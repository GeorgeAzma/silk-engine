use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Weak},
};

use ash::{prelude::VkResult, vk};
use winit::raw_window_handle::{
    HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle,
};

use crate::{prelude::ResultAny, vulkan::physical_device::PhysicalDevice};

/// Create a surface from a raw display and window handle.
///
/// `instance` must have created with platform specific surface extensions enabled, acquired
/// through [`enumerate_required_extensions()`].
///
/// # Safety
///
/// There is a [parent/child relation] between [`Instance`] and [`Entry`], and the resulting
/// [`vk::SurfaceKHR`].  The application must not [destroy][Instance::destroy_instance()] these
/// parent objects before first [destroying][surface::Instance::destroy_surface()] the returned
/// [`vk::SurfaceKHR`] child object.  [`vk::SurfaceKHR`] does _not_ implement [drop][drop()]
/// semantics and can only be destroyed via [`destroy_surface()`][surface::Instance::destroy_surface()].
///
/// See the [`Entry::create_instance()`] documentation for more destruction ordering rules on
/// [`Instance`].
///
/// The window represented by `window_handle` must be associated with the display connection
/// in `display_handle`.
///
/// `window_handle` and `display_handle` must be associated with a valid window and display
/// connection, which must not be destroyed for the lifetime of the returned [`vk::SurfaceKHR`].
///
/// [parent/child relation]: https://registry.khronos.org/vulkan/specs/1.3-extensions/html/vkspec.html#fundamentals-objectmodel-lifetime
pub unsafe fn create_surface(
    entry: &ash::Entry,
    instance: &ash::Instance,
    display_handle: RawDisplayHandle,
    window_handle: RawWindowHandle,
    allocation_callbacks: Option<&vk::AllocationCallbacks<'_>>,
) -> VkResult<vk::SurfaceKHR> {
    match (display_handle, window_handle) {
        (RawDisplayHandle::Windows(_), RawWindowHandle::Win32(window)) => {
            let surface_desc = vk::Win32SurfaceCreateInfoKHR::default()
                .hwnd(window.hwnd.get())
                .hinstance(
                    window
                        .hinstance
                        .ok_or(vk::Result::ERROR_INITIALIZATION_FAILED)?
                        .get(),
                );
            let surface_fn = ash::khr::win32_surface::Instance::new(entry, instance);
            unsafe { surface_fn.create_win32_surface(&surface_desc, allocation_callbacks) }
        }

        (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(window)) => {
            let surface_desc = vk::WaylandSurfaceCreateInfoKHR::default()
                .display(display.display.as_ptr())
                .surface(window.surface.as_ptr());
            let surface_fn = ash::khr::wayland_surface::Instance::new(entry, instance);
            unsafe { surface_fn.create_wayland_surface(&surface_desc, allocation_callbacks) }
        }

        (RawDisplayHandle::Xlib(display), RawWindowHandle::Xlib(window)) => {
            let surface_desc = vk::XlibSurfaceCreateInfoKHR::default()
                .dpy(
                    display
                        .display
                        .ok_or(vk::Result::ERROR_INITIALIZATION_FAILED)?
                        .as_ptr(),
                )
                .window(window.window);
            let surface_fn = ash::khr::xlib_surface::Instance::new(entry, instance);
            unsafe { surface_fn.create_xlib_surface(&surface_desc, allocation_callbacks) }
        }

        (RawDisplayHandle::Xcb(display), RawWindowHandle::Xcb(window)) => {
            let surface_desc = vk::XcbSurfaceCreateInfoKHR::default()
                .connection(
                    display
                        .connection
                        .ok_or(vk::Result::ERROR_INITIALIZATION_FAILED)?
                        .as_ptr(),
                )
                .window(window.window.get());
            let surface_fn = ash::khr::xcb_surface::Instance::new(entry, instance);
            unsafe { surface_fn.create_xcb_surface(&surface_desc, allocation_callbacks) }
        }

        (RawDisplayHandle::Android(_), RawWindowHandle::AndroidNdk(window)) => {
            let surface_desc =
                vk::AndroidSurfaceCreateInfoKHR::default().window(window.a_native_window.as_ptr());
            let surface_fn = ash::khr::android_surface::Instance::new(entry, instance);
            unsafe { surface_fn.create_android_surface(&surface_desc, allocation_callbacks) }
        }

        #[cfg(target_os = "macos")]
        (RawDisplayHandle::AppKit(_), RawWindowHandle::AppKit(window)) => {
            use raw_window_metal::{Layer, appkit};

            let layer = match appkit::metal_layer_from_handle(window) {
                Layer::Existing(layer) | Layer::Allocated(layer) => layer.cast(),
            };

            let surface_desc = vk::MetalSurfaceCreateInfoEXT::default().layer(&*layer);
            let surface_fn = ash::khr::metal_surface::Instance::new(entry, instance);
            surface_fn.create_metal_surface(&surface_desc, allocation_callbacks)
        }

        #[cfg(target_os = "ios")]
        (RawDisplayHandle::UiKit(_), RawWindowHandle::UiKit(window)) => {
            use raw_window_metal::{Layer, uikit};

            let layer = match uikit::metal_layer_from_handle(window) {
                Layer::Existing(layer) | Layer::Allocated(layer) => layer.cast(),
            };

            let surface_desc = vk::MetalSurfaceCreateInfoEXT::default().layer(&*layer);
            let surface_fn = ash::khr::metal_surface::Instance::new(entry, instance);
            surface_fn.create_metal_surface(&surface_desc, allocation_callbacks)
        }

        _ => Err(vk::Result::ERROR_EXTENSION_NOT_PRESENT),
    }
}

pub(crate) struct Surface {
    surface: vk::SurfaceKHR,
    formats: Vec<vk::SurfaceFormatKHR>,
    present_modes: Vec<vk::PresentModeKHR>,
    capabilities: vk::SurfaceCapabilitiesKHR,
    get_surface_capabilities2_instance: ash::khr::get_surface_capabilities2::Instance,
    physical_device: Weak<PhysicalDevice>,
}

impl Deref for Surface {
    type Target = vk::SurfaceKHR;
    fn deref(&self) -> &Self::Target {
        &self.surface
    }
}

impl DerefMut for Surface {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.surface
    }
}

impl Surface {
    pub(crate) fn new(
        physical_device: &Arc<PhysicalDevice>,
        window: &winit::window::Window,
    ) -> ResultAny<Self> {
        let window_handle = window.window_handle()?;
        let display_handle = window.display_handle()?;

        let vulkan = physical_device.vulkan();

        let surface = unsafe {
            create_surface(
                vulkan.entry(),
                vulkan.instance(),
                display_handle.as_raw(),
                window_handle.as_raw(),
                vulkan.allocation_callbacks(),
            )
        }?;

        let formats = physical_device.get_surface_formats(surface)?;
        let present_modes = physical_device.get_surface_present_modes(surface)?;
        let capabilities = physical_device.get_surface_capabilities(surface);

        Ok(Self {
            surface,
            formats,
            present_modes,
            capabilities,
            get_surface_capabilities2_instance: vulkan.get_surface_capabilities2_instance().clone(),
            physical_device: Arc::downgrade(physical_device),
        })
    }

    pub(crate) fn choose_format(
        &self,
        preferred_formats: &[vk::SurfaceFormatKHR],
    ) -> ResultAny<vk::SurfaceFormatKHR> {
        preferred_formats
            .iter()
            .find(|f| self.formats.contains(f))
            .copied()
            .ok_or_else(|| {
                format!("surface does not support preferred surface formats: {preferred_formats:?}")
                    .into()
            })
    }

    pub(crate) fn choose_present_mode(
        &self,
        present_modes: &[vk::PresentModeKHR],
    ) -> ResultAny<vk::PresentModeKHR> {
        present_modes
            .iter()
            .find(|pm| self.present_modes.contains(pm))
            .copied()
            .ok_or_else(|| {
                format!("surface does not support preferred present modes: {present_modes:?}")
                    .into()
            })
    }

    pub(crate) fn capabilities(&self) -> &vk::SurfaceCapabilitiesKHR {
        &self.capabilities
    }

    pub(crate) fn update_capabilities(&mut self) -> &vk::SurfaceCapabilitiesKHR {
        self.capabilities = self
            .physical_device()
            .get_surface_capabilities(self.surface);
        &self.capabilities
    }

    pub(crate) fn physical_device(&self) -> Arc<PhysicalDevice> {
        self.physical_device.upgrade().unwrap()
    }
}
