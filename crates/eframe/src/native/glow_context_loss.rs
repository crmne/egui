//! Reset notification for Windows OpenGL contexts.

#![expect(unsafe_code)]

/// The reset query loaded for one native GL context.
#[derive(Default)]
pub struct ResetStatus {
    #[cfg(target_os = "windows")]
    query: Option<unsafe extern "system" fn() -> u32>,
}

impl ResetStatus {
    /// Load the reset query supported by this context. Unsupported drivers
    /// retain their existing behavior. Currently polls only on Windows.
    ///
    /// # Safety
    /// The context must be current and the loader must belong to that context.
    pub unsafe fn load(
        _gl: &glow::Context,
        mut _load: impl FnMut(&std::ffi::CStr) -> *const std::ffi::c_void,
    ) -> Self {
        #[cfg(target_os = "windows")]
        {
            use glow::HasContext as _;
            let extensions = _gl.supported_extensions();
            let version = _gl.version();
            let core = if version.is_embedded { (3, 2) } else { (4, 5) };
            let name = if (version.major, version.minor) >= core {
                c"glGetGraphicsResetStatus"
            } else if extensions.contains("GL_KHR_robustness") {
                if version.is_embedded {
                    c"glGetGraphicsResetStatusKHR"
                } else {
                    c"glGetGraphicsResetStatus"
                }
            } else if extensions.contains("GL_ARB_robustness") {
                c"glGetGraphicsResetStatusARB"
            } else if extensions.contains("GL_EXT_robustness") {
                c"glGetGraphicsResetStatusEXT"
            } else {
                return Self::default();
            };
            let address = _load(name);
            let query = if address.is_null() {
                None
            } else {
                // SAFETY: the driver advertised the matching API. The pointer
                // belongs to this context and is used only while it is current.
                Some(unsafe {
                    std::mem::transmute::<*const std::ffi::c_void, unsafe extern "system" fn() -> u32>(
                        address,
                    )
                })
            };
            Self { query }
        }
        #[cfg(not(target_os = "windows"))]
        Self::default()
    }

    /// Return [`crate::Error::OpenGLContextLost`] after a GPU reset.
    ///
    /// # Errors
    /// Returns context loss when the driver reports any graphics reset.
    ///
    /// # Safety
    /// The context used by [`Self::load`] must still exist and be current.
    pub unsafe fn check(&self) -> crate::Result<()> {
        #[cfg(target_os = "windows")]
        if let Some(query) = self.query {
            // SAFETY: called on the event thread with this GL context current.
            if unsafe { query() } != glow::NO_ERROR {
                return Err(crate::Error::OpenGLContextLost);
            }
        }
        Ok(())
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    unsafe extern "system" fn reset() -> u32 {
        glow::UNKNOWN_CONTEXT_RESET
    }

    #[test]
    fn reset_becomes_a_distinct_recoverable_error() {
        // SAFETY: these fake queries do not access OpenGL.
        assert!(unsafe { ResetStatus::default().check() }.is_ok());
        assert!(matches!(
            // SAFETY: this fake query does not access OpenGL.
            unsafe { ResetStatus { query: Some(reset) }.check() },
            Err(crate::Error::OpenGLContextLost)
        ));
    }
}
