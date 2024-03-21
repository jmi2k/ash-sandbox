use core::{cell::Cell, ffi::CStr, slice};
use std::array;

use ash::vk;

use crate::window::Window;

pub mod render;
mod wrap;

#[rustfmt::skip]
const INSTANCE_EXTENSIONS: [&CStr; 4] = [
    vk::ExtSwapchainColorspaceFn::NAME,
    vk::KhrPortabilityEnumerationFn::NAME,
    vk::KhrSurfaceFn::NAME,

    #[cfg(windows)]
    vk::KhrWin32SurfaceFn::NAME,

    #[cfg(unix)]
    vk::KhrXlibSurfaceFn::NAME,
];

#[rustfmt::skip]
const DEVICE_EXTENSIONS: [&CStr; 2] = [
    vk::KhrDynamicRenderingFn::NAME,
    vk::KhrSwapchainFn::NAME,
];

pub struct Graphics<'w> {
    recreate_swapchain: Cell<bool>,
    current_frame: usize,

    window: &'w Window,
    instance: wrap::Instance,
    surface: vk::SurfaceKHR,
    physical_device: wrap::PhysicalDevice,
    queue_family: u32,
    device: wrap::Device,
    queue: vk::Queue,
    command_pool: vk::CommandPool,
    swapchain: wrap::Swapchain,

    fifs: [(vk::CommandBuffer, vk::Fence, vk::Semaphore); 2],
}

impl<'win> Graphics<'win> {
    pub fn new(window: &'win Window) -> Self {
        // jmi2k: TODO: enable features.
        // jmi2k: TODO: test features.
        // jmi2k: mark command pool as TRANSIENT?

        let app_info = vk::ApplicationInfo::default()
            .api_version(vk::API_VERSION_1_2)
            .application_name(c"ash-sandbox")
            .engine_name(c"picon");

        let instance_extensions = INSTANCE_EXTENSIONS.map(CStr::as_ptr);
        let instance_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&instance_extensions)
            .enabled_layer_names(&[])
            .flags(vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR);

        let instance = unsafe { wrap::Instance::new(ash::Entry::linked(), &instance_info) }
            .expect("Failed to create instance");

        #[cfg(windows)]
        #[rustfmt::skip]
        let surface = unsafe { instance.create_win32_surface(window) }
            .expect("Failed to create surface");

        #[cfg(unix)]
        #[rustfmt::skip]
        let surface = unsafe { instance.create_xlib_surface(window) }
            .expect("Failed to create surface");

        let mut physical_devices = unsafe { wrap::PhysicalDevice::enumerate(&instance) }
            .expect("Failed to enumerate physical devices")
            .filter_map(filter_physical_device)
            .filter(|(device, idx)| unsafe { device.supports_surface(&instance, *idx, surface) })
            .collect::<Vec<_>>();

        physical_devices.sort_by_key(|(device, _)| rank_physical_device(device));

        let (physical_device, queue_family) = physical_devices
            .into_iter()
            .next()
            .expect("No compatible physical device found");

        let queue_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family)
            .queue_priorities(&[1.]);

        #[rustfmt::skip]
        let mut dynamic_rendering_info = vk::PhysicalDeviceDynamicRenderingFeaturesKHR::default()
            .dynamic_rendering(true);

        let device_exts = DEVICE_EXTENSIONS.map(CStr::as_ptr);
        let device_info = vk::DeviceCreateInfo::default()
            .enabled_extension_names(&device_exts)
            .queue_create_infos(slice::from_ref(&queue_info))
            .push_next(&mut dynamic_rendering_info);

        let device = unsafe { wrap::Device::new(&instance, &physical_device, &device_info) }
            .expect("Failed to create device");

        let queue = unsafe { device.get_device_queue(queue_family, 0) };

        let commands_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family);

        let command_pool = unsafe { device.create_command_pool(&commands_info, None) }
            .expect("Failed to create command pool");

        let commands_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .command_buffer_count(2)
            .level(vk::CommandBufferLevel::PRIMARY);

        let command_buffers = unsafe { device.allocate_command_buffers(&commands_info) }
            .expect("Failed to allocate command buffers");

        let fifs = array::from_fn(|idx| {
            let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

            let available = unsafe { device.create_fence(&fence_info, None) }
                .expect("Failed to allocate FIF fence");

            let acquired = unsafe { device.create_semaphore(&Default::default(), None) }
                .expect("Failed to create FIF semaphore");

            (command_buffers[idx], available, acquired)
        });

        Self {
            recreate_swapchain: true.into(),
            current_frame: 0,

            window,
            instance,
            surface,
            physical_device,
            queue_family,
            device,
            queue,
            command_pool,
            swapchain: Default::default(),
            fifs,
        }
    }

    pub fn invalidate_swapchain(&self) {
        self.recreate_swapchain.set(true);
    }

    pub fn prepare_frame(&mut self, mut callback: impl FnMut(Frame)) {
        if self.recreate_swapchain.get() {
            unsafe { self.recreate_swapchain() };
        }

        let (commands, available, acquired) = self.fifs[self.current_frame & 1];
        unsafe { self.device.wait_for_fences(&[available], true, u64::MAX) }.unwrap();

        #[rustfmt::skip]
        let acquire_result = unsafe { self.device.acquire_image(&self.swapchain, acquired) };

        let (idx, bad) = match acquire_result {
            Ok(res) => res,

            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.invalidate_swapchain();
                return;
            }

            _ => panic!("Failed to acquire image"),
        };

        if bad {
            self.invalidate_swapchain();
        }

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe { self.device.reset_fences(&[available]) }.unwrap();

        unsafe { self.device.begin_command_buffer(commands, &begin_info) }
            .expect("Failed to begin recording command buffer");

        let (image, view, rendered) = self.swapchain.image(idx);

        callback(Frame {
            device: &self.device,
            image,
            view,
            extent: self.swapchain.extent(),
            format: self.swapchain.format(),
            commands,
        });

        let submit_info = vk::SubmitInfo::default()
            .command_buffers(slice::from_ref(&commands))
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .wait_semaphores(slice::from_ref(&acquired))
            .signal_semaphores(slice::from_ref(&rendered));

        let present_info = vk::PresentInfoKHR::default()
            .image_indices(slice::from_ref(&idx))
            .swapchains(slice::from_ref(&self.swapchain))
            .wait_semaphores(slice::from_ref(&rendered));

        unsafe { self.device.end_command_buffer(commands) }
            .expect("Failed to finish recording command buffer");

        #[rustfmt::skip]
        unsafe { self.device.queue_submit(self.queue, &[submit_info], available) }
            .expect("Failed to submit commands to queue");

        match unsafe { self.device.present(self.queue, &present_info) } {
            Ok(_) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {}
            _ => panic!("Failed to present image"),
        }

        self.current_frame += 1;
    }

    unsafe fn recreate_swapchain(&mut self) -> &wrap::Swapchain {
        // jmi2k: TODO: allow selecting present mode.

        let present_mode = self
            .instance
            .surface_present_modes(&self.physical_device, &self.surface)
            .expect("Failed to get present modes")
            .into_iter()
            .find(|mode| *mode == vk::PresentModeKHR::IMMEDIATE)
            .unwrap();

        let vk::SurfaceCapabilitiesKHR {
            min_image_count,
            mut max_image_count,
            current_extent,
            ..
        } = self
            .instance
            .surface_capabilities(&self.physical_device, &self.surface)
            .expect("Failed to get surface capabilities");

        let vk::SurfaceFormatKHR {
            format,
            color_space,
        } = self
            .instance
            .surface_formats(&self.physical_device, &self.surface)
            .expect("Failed to get surface formats")
            .into_iter()
            .min_by_key(rank_surface_format)
            .unwrap();

        if max_image_count == 0 {
            max_image_count = u32::MAX;
        }

        let vk::Extent2D {
            mut width,
            mut height,
        } = current_extent;

        if (width & height) == u32::MAX {
            [width, height] = self.window.inner_size();
        }

        let swapchain_info = vk::SwapchainCreateInfoKHR::default()
            .surface(self.surface)
            .image_format(format)
            .image_color_space(color_space)
            .image_extent(vk::Extent2D { width, height })
            .min_image_count(u32::clamp(3, min_image_count, max_image_count))
            .present_mode(present_mode)
            .queue_family_indices(slice::from_ref(&self.queue_family));

        let new_swapchain = self
            .swapchain
            .recreate(&self.device, swapchain_info)
            .expect("Failed to recreate swapchain");

        self.recreate_swapchain.set(false);
        self.swapchain = new_swapchain;

        &self.swapchain
    }
}

impl Drop for Graphics<'_> {
    fn drop(&mut self) {
        unsafe {
            _ = self.device.device_wait_idle();

            self.swapchain.teardown(&self.device);
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_device(None);
            self.instance.destroy_surface(self.surface);
            self.instance.destroy_instance(None);
        }
    }
}

pub struct Frame<'g> {
    device: &'g wrap::Device,
    image: vk::Image,
    view: vk::ImageView,
    extent: vk::Extent2D,
    format: vk::Format,
    commands: vk::CommandBuffer,
}

pub fn is_srgb(format: vk::Format) -> bool {
    #[rustfmt::skip]
    matches!(format,
        | vk::Format::R8_SRGB
        | vk::Format::R8G8_SRGB
        | vk::Format::R8G8B8_SRGB
        | vk::Format::B8G8R8_SRGB
        | vk::Format::R8G8B8A8_SRGB
        | vk::Format::B8G8R8A8_SRGB
        | vk::Format::A8B8G8R8_SRGB_PACK32
        | vk::Format::BC1_RGB_SRGB_BLOCK
        | vk::Format::BC1_RGBA_SRGB_BLOCK
        | vk::Format::BC2_SRGB_BLOCK
        | vk::Format::BC3_SRGB_BLOCK
        | vk::Format::BC7_SRGB_BLOCK
        | vk::Format::ETC2_R8G8B8_SRGB_BLOCK
        | vk::Format::ETC2_R8G8B8A1_SRGB_BLOCK
        | vk::Format::ETC2_R8G8B8A8_SRGB_BLOCK
        | vk::Format::ASTC_4X4_SRGB_BLOCK
        | vk::Format::ASTC_5X4_SRGB_BLOCK
        | vk::Format::ASTC_5X5_SRGB_BLOCK
        | vk::Format::ASTC_6X5_SRGB_BLOCK
        | vk::Format::ASTC_6X6_SRGB_BLOCK
        | vk::Format::ASTC_8X5_SRGB_BLOCK
        | vk::Format::ASTC_8X6_SRGB_BLOCK
        | vk::Format::ASTC_8X8_SRGB_BLOCK
        | vk::Format::ASTC_10X5_SRGB_BLOCK
        | vk::Format::ASTC_10X6_SRGB_BLOCK
        | vk::Format::ASTC_10X8_SRGB_BLOCK
        | vk::Format::ASTC_10X10_SRGB_BLOCK
        | vk::Format::ASTC_12X10_SRGB_BLOCK
        | vk::Format::ASTC_12X12_SRGB_BLOCK)
}

fn filter_physical_device(device: wrap::PhysicalDevice) -> Option<(wrap::PhysicalDevice, u32)> {
    let compatible_features = true;

    if !compatible_features {
        return None;
    }

    let required_family_flags = vk::QueueFlags::empty()
        | vk::QueueFlags::COMPUTE
        | vk::QueueFlags::GRAPHICS
        | vk::QueueFlags::TRANSFER;

    device
        .queue_families
        .iter()
        .position(|props| props.queue_flags.intersects(required_family_flags))
        .map(|idx| (device, idx as _))
}

fn rank_physical_device(device: &wrap::PhysicalDevice) -> usize {
    match device.properties.device_type {
        // Usually the discrete GPU is the most powerful one.
        vk::PhysicalDeviceType::DISCRETE_GPU => 0,
        // An integrated GPU is also acceptable.
        vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
        // Any HW acceleration is better than no HW acceleration.
        vk::PhysicalDeviceType::VIRTUAL_GPU => 2,
        // Something like `llvmpipe` will be barely usable.
        vk::PhysicalDeviceType::CPU => 3,
        // Leave the unknown as a last resort option.
        vk::PhysicalDeviceType::OTHER => 4,
        _ => usize::MAX,
    }
}

fn rank_surface_format(surface_format: &vk::SurfaceFormatKHR) -> usize {
    match surface_format.format {
        // Prefer 10-bit formats to avoid banding.
        vk::Format::A2B10G10R10_UNORM_PACK32 => 0,
        vk::Format::A2R10G10B10_UNORM_PACK32 => 0,
        // 8bpc sRGB formats are preferrable.
        vk::Format::R8G8B8A8_SRGB => 1,
        vk::Format::B8G8R8A8_SRGB => 1,
        // Any format is better than no format.
        _ => usize::MAX,
    }
}
