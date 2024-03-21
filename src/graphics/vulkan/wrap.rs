use ash::{extensions::khr, prelude::VkResult, vk};

use crate::{utils, window::Window};

pub struct Instance {
    inner: ash::Instance,
    entry: ash::Entry,
    surface_loader: khr::Surface,
}

utils::wrap! { Instance, ash::Instance }

impl Instance {
    pub unsafe fn new(entry: ash::Entry, info: &vk::InstanceCreateInfo) -> VkResult<Self> {
        let inner = entry.create_instance(info, None)?;
        let surface_loader = khr::Surface::new(&entry, &inner);

        Ok(Self {
            inner,
            entry,
            surface_loader,
        })
    }

    #[cfg(windows)]
    pub unsafe fn create_win32_surface(&self, window: &Window) -> VkResult<vk::SurfaceKHR> {
        let info = vk::Win32SurfaceCreateInfoKHR::default()
            .hinstance(window.instance().0)
            .hwnd(window.0);

        khr::Win32Surface::new(&self.entry, self).create_win32_surface(&info, None)
    }

    #[cfg(unix)]
    pub unsafe fn create_xlib_surface(&self, window: &Window) -> VkResult<vk::SurfaceKHR> {
        let info = vk::XlibSurfaceCreateInfoKHR::default()
            .dpy(window.display() as *mut _)
            .window(**window);

        khr::XlibSurface::new(&self.entry, self).create_xlib_surface(&info, None)
    }

    pub unsafe fn surface_present_modes(
        &self,
        physical_device: &PhysicalDevice,
        surface: &vk::SurfaceKHR,
    ) -> VkResult<Vec<vk::PresentModeKHR>> {
        #[rustfmt::skip]
        self.surface_loader.get_physical_device_surface_present_modes(**physical_device, *surface)
    }

    pub unsafe fn surface_capabilities(
        &self,
        physical_device: &PhysicalDevice,
        surface: &vk::SurfaceKHR,
    ) -> VkResult<vk::SurfaceCapabilitiesKHR> {
        #[rustfmt::skip]
        self.surface_loader.get_physical_device_surface_capabilities(**physical_device, *surface)
    }

    pub unsafe fn surface_formats(
        &self,
        physical_device: &PhysicalDevice,
        surface: &vk::SurfaceKHR,
    ) -> VkResult<Vec<vk::SurfaceFormatKHR>> {
        #[rustfmt::skip]
        self.surface_loader.get_physical_device_surface_formats(**physical_device, *surface)
    }

    pub unsafe fn destroy_surface(&self, surface: vk::SurfaceKHR) {
        self.surface_loader.destroy_surface(surface, None);
    }
}

pub struct PhysicalDevice {
    pub inner: vk::PhysicalDevice,
    pub properties: vk::PhysicalDeviceProperties,
    pub queue_families: Box<[vk::QueueFamilyProperties]>,
}

utils::wrap! { PhysicalDevice, vk::PhysicalDevice }

impl PhysicalDevice {
    pub unsafe fn enumerate(instance: &Instance) -> VkResult<impl Iterator<Item = Self> + '_> {
        let inners = instance.enumerate_physical_devices()?;

        let iter = inners.into_iter().map(|inner| {
            let properties = instance.get_physical_device_properties(inner);

            let queue_families = instance
                .get_physical_device_queue_family_properties(inner)
                .into();

            Self {
                inner,
                queue_families,
                properties,
            }
        });

        Ok(iter)
    }

    pub unsafe fn supports_surface(
        &self,
        instance: &Instance,
        queue_family: u32,
        surface: vk::SurfaceKHR,
    ) -> bool {
        instance
            .surface_loader
            .get_physical_device_surface_support(**self, queue_family, surface)
            .unwrap_or_default()
    }
}

pub struct Device {
    inner: ash::Device,
    swapchain_loader: khr::Swapchain,
}

utils::wrap! { Device, ash::Device }

impl Device {
    pub unsafe fn new(
        instance: &Instance,
        physical_device: &PhysicalDevice,
        info: &vk::DeviceCreateInfo,
    ) -> VkResult<Self> {
        let inner = instance.create_device(**physical_device, info, None)?;
        let swapchain_loader = khr::Swapchain::new(instance, &inner);

        Ok(Self {
            inner,
            swapchain_loader,
        })
    }

    pub unsafe fn acquire_image(
        &self,
        swapchain: &Swapchain,
        acquired: vk::Semaphore,
    ) -> VkResult<(u32, bool)> {
        #[rustfmt::skip]
        self.swapchain_loader.acquire_next_image(**swapchain, u64::MAX, acquired, vk::Fence::null())
    }

    pub unsafe fn present(&self, queue: vk::Queue, info: &vk::PresentInfoKHR) -> VkResult<bool> {
        self.swapchain_loader.queue_present(queue, info)
    }

    pub unsafe fn destroy_swapchain(&self, swapchain: &Swapchain) {
        #[rustfmt::skip]
        self.swapchain_loader.destroy_swapchain(**swapchain, None);
    }
}

#[derive(Default)]
pub struct Swapchain {
    inner: vk::SwapchainKHR,
    extent: vk::Extent2D,
    format: vk::Format,
    images: Vec<(vk::Image, vk::ImageView, vk::Semaphore)>,
}

utils::wrap! { Swapchain, vk::SwapchainKHR }

impl Swapchain {
    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }

    pub fn format(&self) -> vk::Format {
        self.format
    }

    pub fn image(&self, idx: u32) -> (vk::Image, vk::ImageView, vk::Semaphore) {
        self.images[idx as usize]
    }

    pub unsafe fn recreate(
        &self,
        device: &Device,
        mut swapchain_info: vk::SwapchainCreateInfoKHR,
    ) -> VkResult<Self> {
        swapchain_info = swapchain_info
            .clipped(true)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .image_array_layers(1)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .old_swapchain(self.inner)
            .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY);

        let inner = device
            .swapchain_loader
            .create_swapchain(&swapchain_info, None)?;

        let bare_images = device.swapchain_loader.get_swapchain_images(inner)?;
        let mut images = Vec::with_capacity(bare_images.len());

        let subrange = vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .level_count(1)
            .layer_count(1);

        for bare_image in bare_images {
            let info = vk::ImageViewCreateInfo::default()
                .image(bare_image)
                .format(swapchain_info.image_format)
                .view_type(vk::ImageViewType::TYPE_2D)
                .subresource_range(subrange);

            let view = device.create_image_view(&info, None)?;
            let rendered = device.create_semaphore(&Default::default(), None)?;

            images.push((bare_image, view, rendered));
        }

        if self.inner != vk::SwapchainKHR::null() {
            device.device_wait_idle().unwrap();
            self.teardown(device);
        }

        Ok(Self {
            inner,
            extent: swapchain_info.image_extent,
            format: swapchain_info.image_format,
            images,
        })
    }

    pub unsafe fn teardown(&self, device: &Device) {
        for (_, view, acquired) in &self.images {
            device.destroy_image_view(*view, None);
            device.destroy_semaphore(*acquired, None);
        }

        device.destroy_swapchain(self);
    }
}
