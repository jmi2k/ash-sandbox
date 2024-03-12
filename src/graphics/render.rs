use core::slice;

use ash::vk;

use crate::utils;

use super::{Frame, Graphics};

pub struct Renderer {
    layout: vk::PipelineLayout,
    pipe: vk::Pipeline,
}

impl Renderer {
    pub fn new(gfx: &Graphics) -> Self {
        let constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(4);

        let layout_info = vk::PipelineLayoutCreateInfo::default()
            .push_constant_ranges(slice::from_ref(&constant_range));

        let layout = unsafe { gfx.device.create_pipeline_layout(&layout_info, None) }
            .expect("Failed to create pipeline layout");

        let vert = unsafe { utils::include_spv!(gfx.device, "../../res/draw.vert.spv") };
        let frag = unsafe { utils::include_spv!(gfx.device, "../../res/draw.frag.spv") };

        let stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .module(vert)
                .name(c"main")
                .stage(vk::ShaderStageFlags::VERTEX),
            vk::PipelineShaderStageCreateInfo::default()
                .module(frag)
                .name(c"main")
                .stage(vk::ShaderStageFlags::FRAGMENT),
        ];

        let vertex_input_info = Default::default();

        let viewport_info = vk::PipelineViewportStateCreateInfo::default()
            .scissor_count(1)
            .viewport_count(1);

        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

        let rasterization_info = vk::PipelineRasterizationStateCreateInfo::default()
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.);

        let multisample_info = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let color_attachment = vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(false)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD);

        let color_blend_info = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(slice::from_ref(&color_attachment))
            .blend_constants([0., 0., 0., 0.]);

        let dynamic_state_info = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&[vk::DynamicState::SCISSOR, vk::DynamicState::VIEWPORT]);

        let mut rendering_info = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(&[vk::Format::A2B10G10R10_UNORM_PACK32]);

        let pipe_info = vk::GraphicsPipelineCreateInfo::default()
            .layout(layout)
            .stages(&stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly_info)
            .viewport_state(&viewport_info)
            .rasterization_state(&rasterization_info)
            .multisample_state(&multisample_info)
            .color_blend_state(&color_blend_info)
            .dynamic_state(&dynamic_state_info)
            .push_next(&mut rendering_info);

        let cache = Default::default();

        #[rustfmt::skip]
        let pipe = unsafe { gfx.device.create_graphics_pipelines(cache, &[pipe_info], None) }
            .expect("Failed to create pipeline layout")[0];

        Self { layout, pipe }
    }

    pub fn render(&self, frame: Frame) {
        let Frame {
            device,
            image,
            view,
            extent,
            format,
            commands,
        } = frame;

        let whole_rect = vk::Rect2D::default().extent(extent);

        let viewport = vk::Viewport::default()
            .width(extent.width as _)
            .height(extent.height as _)
            .max_depth(1.);

        let color_attachment = vk::RenderingAttachmentInfo::default()
            .image_view(view)
            .image_layout(vk::ImageLayout::ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(Default::default());

        let rendering_info = vk::RenderingInfo::default()
            .color_attachments(slice::from_ref(&color_attachment))
            .layer_count(1)
            .render_area(whole_rect);

        let color_subrange = vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .level_count(1)
            .layer_count(1);

        let push_constants = super::is_srgb(format) as u32;

        unsafe {
            let barrier_info = vk::ImageMemoryBarrier::default()
                .image(image)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .subresource_range(color_subrange);

            device.cmd_pipeline_barrier(
                commands,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                Default::default(),
                &[],
                &[],
                &[barrier_info],
            );

            device.cmd_push_constants(
                commands,
                self.layout,
                vk::ShaderStageFlags::FRAGMENT,
                0,
                &push_constants.to_ne_bytes(),
            );

            device.cmd_set_scissor(commands, 0, &[whole_rect]);
            device.cmd_set_viewport(commands, 0, &[viewport]);
            device.cmd_begin_rendering(commands, &rendering_info);
            device.cmd_bind_pipeline(commands, vk::PipelineBindPoint::GRAPHICS, self.pipe);
            device.cmd_draw(commands, 3, 1, 0, 0);
            device.cmd_end_rendering(commands);

            let barrier_info = vk::ImageMemoryBarrier::default()
                .image(image)
                .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .subresource_range(color_subrange);

            device.cmd_pipeline_barrier(
                commands,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                Default::default(),
                &[],
                &[],
                &[barrier_info],
            );
        }
    }
}
