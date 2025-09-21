use anyhow::{anyhow, Result};
use khronos_egl as egl;
use std::ffi::c_void;
use std::ptr;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::Proxy;

pub struct EglContext {
    _egl: egl::Instance<egl::Static>,
    display: egl::Display,
    context: egl::Context,
    config: egl::Config,
    wl_egl_window: Option<wayland_egl::WlEglSurface>,
    surface: Option<egl::Surface>,
    owns_display: bool,  // Whether this context owns the display (should terminate on drop)
}

impl EglContext {
    pub fn new(wl_display: *mut c_void) -> Result<Self> {
        let egl_instance = egl::Instance::new(egl::Static);

        // Initialize EGL
        let display = unsafe {
            egl_instance.get_display(wl_display as egl::NativeDisplayType)
                .ok_or_else(|| anyhow!("Failed to get EGL display"))?
        };

        let (major, minor) = egl_instance.initialize(display)?;
        log::info!("EGL version: {}.{}", major, minor);

        let config_attribs = [
            egl::SURFACE_TYPE, egl::WINDOW_BIT,
            egl::RED_SIZE, 8,
            egl::GREEN_SIZE, 8,
            egl::BLUE_SIZE, 8,
            egl::ALPHA_SIZE, 8,
            egl::RENDERABLE_TYPE, egl::OPENGL_ES2_BIT,
            egl::NONE,
        ];

        let config = egl_instance.choose_first_config(display, &config_attribs)?
            .ok_or_else(|| anyhow!("No EGL config found"))?;

        egl_instance.bind_api(egl::OPENGL_ES_API)?;

        let context_attribs = [
            egl::CONTEXT_CLIENT_VERSION, 2,
            egl::NONE,
        ];

        let context = egl_instance.create_context(display, config, None, &context_attribs)?;

        Ok(Self {
            _egl: egl_instance,
            display,
            context,
            config,
            wl_egl_window: None,
            surface: None,
            owns_display: true,  // First context owns the display
        })
    }

    pub fn create_surface(&mut self, wl_surface: &WlSurface, width: i32, height: i32) -> Result<()> {
        // Clean up existing surface if any
        if let Some(surface) = self.surface.take() {
            unsafe {
                self._egl.destroy_surface(self.display, surface)?;
            }
        }

        // Drop old window if exists
        self.wl_egl_window = None;

        // Create new wl_egl_window
        let wl_egl_window = unsafe {
            wayland_egl::WlEglSurface::new_from_raw(
                wl_surface.id().as_ptr() as *mut _,
                width,
                height,
            )?
        };

        // Create EGL surface
        let surface = unsafe {
            self._egl.create_window_surface(
                self.display,
                self.config,
                wl_egl_window.ptr() as egl::NativeWindowType,
                None,
            )?
        };

        self.wl_egl_window = Some(wl_egl_window);
        self.surface = Some(surface);

        // Make current
        unsafe {
            self._egl.make_current(
                self.display,
                Some(surface),
                Some(surface),
                Some(self.context),
            )?;
        }

        Ok(())
    }

    pub fn resize(&mut self, width: i32, height: i32) -> Result<()> {
        if let Some(window) = &mut self.wl_egl_window {
            window.resize(width, height, 0, 0);
        }
        Ok(())
    }

    pub fn swap_buffers(&self) -> Result<()> {
        if let Some(surface) = self.surface {
            unsafe {
                self._egl.swap_buffers(self.display, surface)?;
            }
        }
        Ok(())
    }

    pub fn make_current(&self) -> Result<()> {
        if let Some(surface) = self.surface {
            unsafe {
                self._egl.make_current(
                    self.display,
                    Some(surface),
                    Some(surface),
                    Some(self.context),
                )?;
            }
        }
        Ok(())
    }

    pub fn get_proc_address(&self, name: &str) -> *const c_void {
        self._egl.get_proc_address(name)
            .map(|f| f as *const c_void)
            .unwrap_or(ptr::null())
    }

    /// Create a new EGL context sharing the same display (for secondary windows)
    /// The returned context will NOT terminate the display when dropped
    pub fn new_shared(wl_display: *mut c_void) -> Result<Self> {
        let egl_instance = egl::Instance::new(egl::Static);

        // Get the same display (won't be initialized again)
        let display = unsafe {
            egl_instance.get_display(wl_display as egl::NativeDisplayType)
                .ok_or_else(|| anyhow!("Failed to get EGL display"))?
        };

        // Note: display is already initialized by the first context

        let config_attribs = [
            egl::SURFACE_TYPE, egl::WINDOW_BIT,
            egl::RED_SIZE, 8,
            egl::GREEN_SIZE, 8,
            egl::BLUE_SIZE, 8,
            egl::ALPHA_SIZE, 8,
            egl::RENDERABLE_TYPE, egl::OPENGL_ES2_BIT,
            egl::NONE,
        ];

        let config = egl_instance.choose_first_config(display, &config_attribs)?
            .ok_or_else(|| anyhow!("No EGL config found"))?;

        egl_instance.bind_api(egl::OPENGL_ES_API)?;

        let context_attribs = [
            egl::CONTEXT_CLIENT_VERSION, 2,
            egl::NONE,
        ];

        let context = egl_instance.create_context(display, config, None, &context_attribs)?;

        Ok(Self {
            _egl: egl_instance,
            display,
            context,
            config,
            wl_egl_window: None,
            surface: None,
            owns_display: false,  // Secondary context doesn't own the display
        })
    }
}

impl Drop for EglContext {
    fn drop(&mut self) {
        unsafe {
            let _ = self._egl.make_current(self.display, None, None, None);

            if let Some(surface) = self.surface {
                let _ = self._egl.destroy_surface(self.display, surface);
            }

            let _ = self._egl.destroy_context(self.display, self.context);

            // Only terminate display if we own it
            if self.owns_display {
                let _ = self._egl.terminate(self.display);
            }
        }
    }
}