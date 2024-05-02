#![allow(unsafe_code)]

use bevy_ecs::prelude::Component;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};
use std::{fmt, ops::Deref, sync::Arc};

/// A wrapper over a window.
///
/// This allows us to extend the lifetime of the window, so it doesn't get eagerly dropped while a
/// pipelined renderer still has frames in flight that need to draw to it.
///
/// This is achieved by storing a shared reference to the window in the [`RawHandleWrapper`],
/// which gets picked up by the renderer during extraction.
#[derive(Debug)]
pub struct WindowWrapper<W> {
    reference: Arc<W>,
}

impl<W: Send + Sync + 'static> WindowWrapper<W> {
    /// Creates a `WindowWrapper` from a window.
    pub fn new(window: W) -> WindowWrapper<W> {
        WindowWrapper {
            reference: Arc::new(window),
        }
    }
}

impl<W: 'static> Deref for WindowWrapper<W> {
    type Target = W;

    fn deref(&self) -> &Self::Target {
        &self.reference
    }
}

trait WindowTrait: HasWindowHandle + HasDisplayHandle {}
impl<T: HasWindowHandle + HasDisplayHandle> WindowTrait for T {}

/// A wrapper over [`HasWindowHandle`] and [`HasDisplayHandle`] that allows us to safely pass it across threads.
///
/// Depending on the platform, the underlying pointer-containing handle cannot be used on all threads,
/// and so we cannot simply make it (or any type that has a safe operation to get a [`WindowHandle`] or [`DisplayHandle`])
/// thread-safe.
#[derive(Clone, Component)]
pub struct RawHandleWrapper {
    window: Arc<dyn WindowTrait>,
}

impl fmt::Debug for RawHandleWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_struct("RawHandleWrapper").finish_non_exhaustive()
    }
}

impl RawHandleWrapper {
    /// Creates a `RawHandleWrapper` from a `WindowWrapper`.
    pub fn new<W: HasWindowHandle + HasDisplayHandle + 'static>(
        window: &WindowWrapper<W>,
    ) -> Result<RawHandleWrapper, HandleError> {
        Ok(RawHandleWrapper {
            window: window.reference.clone(),
        })
    }

    /// Returns a [`HasWindowHandle`] + [`HasDisplayHandle`] impl, which exposes [`WindowHandle`] and [`DisplayHandle`].
    ///
    /// # Safety
    ///
    /// Some platforms have constraints on where/how this handle can be used. For example, some platforms don't support doing window
    /// operations off of the main thread. The caller must ensure the [`RawHandleWrapper`] is only used in valid contexts.
    pub unsafe fn get_handle(&self) -> ThreadLockedRawWindowHandleWrapper {
        ThreadLockedRawWindowHandleWrapper(self.clone())
    }
}

// SAFETY: [`RawHandleWrapper`] is just a normal "raw pointer", which doesn't impl Send/Sync. However the pointer is only
// exposed via an unsafe method that forces the user to make a call for a given platform. (ex: some platforms don't
// support doing window operations off of the main thread).
// A recommendation for this pattern (and more context) is available here:
// https://github.com/rust-windowing/raw-window-handle/issues/59
unsafe impl Send for RawHandleWrapper {}
// SAFETY: This is safe for the same reasons as the Send impl above.
unsafe impl Sync for RawHandleWrapper {}

/// A [`RawHandleWrapper`] that cannot be sent across threads.
///
/// This safely exposes [`RawWindowHandle`] and [`RawDisplayHandle`], but care must be taken to ensure that the construction itself is correct.
///
/// This can only be constructed via the [`RawHandleWrapper::get_handle()`] method;
/// be sure to read the safety docs there about platform-specific limitations.
/// In many cases, this should only be constructed on the main thread.
pub struct ThreadLockedRawWindowHandleWrapper(RawHandleWrapper);

impl HasWindowHandle for ThreadLockedRawWindowHandleWrapper {
    fn window_handle(&self) -> Result<WindowHandle, HandleError> {
        // SAFETY: the caller has validated that this is a valid context to get [`RawHandleWrapper`]
        // as otherwise an instance of this type could not have been constructed
        // NOTE: we cannot simply impl HasRawWindowHandle for RawHandleWrapper,
        // as the `raw_window_handle` method is safe. We cannot guarantee that all calls
        // of this method are correct (as it may be off the main thread on an incompatible platform),
        // and so exposing a safe method to get a [`RawWindowHandle`] directly would be UB.
        self.0.window.window_handle()
    }
}

impl HasDisplayHandle for ThreadLockedRawWindowHandleWrapper {
    fn display_handle(&self) -> Result<DisplayHandle, HandleError> {
        // SAFETY: the caller has validated that this is a valid context to get [`RawDisplayHandle`]
        // as otherwise an instance of this type could not have been constructed
        // NOTE: we cannot simply impl HasRawDisplayHandle for RawHandleWrapper,
        // as the `raw_display_handle` method is safe. We cannot guarantee that all calls
        // of this method are correct (as it may be off the main thread on an incompatible platform),
        // and so exposing a safe method to get a [`RawDisplayHandle`] directly would be UB.
        self.0.window.display_handle()
    }
}
