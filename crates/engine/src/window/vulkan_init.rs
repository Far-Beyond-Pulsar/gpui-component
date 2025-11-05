//! Vulkan Initialization Module
//!
//! This module provides Vulkan initialization for Linux and macOS platforms,
//! equivalent to the D3D11 initialization on Windows.
//!
//! ## Architecture
//!
//! The Vulkan renderer performs 3-layer composition:
//! - Layer 0 (bottom): Clear color background
//! - Layer 1 (middle): Bevy 3D rendering (opaque)
//! - Layer 2 (top): GPUI UI (alpha-blended)

#![cfg(not(target_os = "windows"))]

use super::state::VulkanState;
use anyhow::{Context, Result};
use ash::vk;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::ffi::CStr;
use std::sync::Arc;

/// Vulkan shader code (SPIR-V) for fullscreen quad vertex shader
const VERT_SHADER_SPIRV: &[u8] = include_bytes!("shaders/fullscreen.vert.spv");

/// Vulkan shader code (SPIR-V) for texture sampling fragment shader
const FRAG_SHADER_SPIRV: &[u8] = include_bytes!("shaders/texture.frag.spv");

/// Initialize Vulkan for a window
pub unsafe fn init_vulkan(
    window: &winit::window::Window,
    width: u32,
    height: u32,
) -> Result<VulkanState> {
    println!("🔵 Initializing Vulkan renderer...");

    // 1. Create Vulkan entry and instance
    let entry = unsafe { ash::Entry::load()? };

    let app_name = CStr::from_bytes_with_nul_unchecked(b"Pulsar Engine\0");
    let engine_name = CStr::from_bytes_with_nul_unchecked(b"Pulsar\0");

    let app_info = vk::ApplicationInfo::default()
        .application_name(app_name)
        .application_version(vk::make_api_version(0, 1, 0, 0))
        .engine_name(engine_name)
        .engine_version(vk::make_api_version(0, 1, 0, 0))
        .api_version(vk::API_VERSION_1_2);

    // Get required extensions for window surface
    let mut extension_names = ash_window::enumerate_required_extensions(
        window.display_handle().unwrap().as_raw()
    )?
    .to_vec();

    // Add debug utils extension in debug mode
    #[cfg(debug_assertions)]
    extension_names.push(ash::ext::debug_utils::NAME.as_ptr());

    let create_info = vk::InstanceCreateInfo::default()
        .application_info(&app_info)
        .enabled_extension_names(&extension_names);

    let instance = entry.create_instance(&create_info, None)
        .context("Failed to create Vulkan instance")?;

    println!("✅ Created Vulkan instance");

    // 2. Create window surface
    let surface = ash_window::create_surface(
        &entry,
        &instance,
        window.display_handle().unwrap().as_raw(),
        window.window_handle().unwrap().as_raw(),
        None,
    )?;

    let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);

    println!("✅ Created window surface");

    // 3. Select physical device
    let physical_devices = instance.enumerate_physical_devices()?;
    let (physical_device, graphics_queue_family) = physical_devices
        .into_iter()
        .find_map(|pdevice| {
            let queue_families = instance.get_physical_device_queue_family_properties(pdevice);
            queue_families
                .iter()
                .enumerate()
                .find_map(|(index, info)| {
                    let supports_graphics = info.queue_flags.contains(vk::QueueFlags::GRAPHICS);
                    let supports_surface = surface_loader
                        .get_physical_device_surface_support(pdevice, index as u32, surface)
                        .unwrap_or(false);
                    if supports_graphics && supports_surface {
                        Some((pdevice, index as u32))
                    } else {
                        None
                    }
                })
        })
        .context("Failed to find suitable physical device")?;

    let device_properties = instance.get_physical_device_properties(physical_device);
    let device_name = CStr::from_ptr(device_properties.device_name.as_ptr());
    println!("✅ Selected physical device: {:?}", device_name);

    // 4. Create logical device
    let queue_priorities = [1.0f32];
    let queue_create_info = vk::DeviceQueueCreateInfo::default()
        .queue_family_index(graphics_queue_family)
        .queue_priorities(&queue_priorities);

    let device_extension_names = [ash::khr::swapchain::NAME.as_ptr()];

    let device_create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(std::slice::from_ref(&queue_create_info))
        .enabled_extension_names(&device_extension_names);

    let device = instance.create_device(physical_device, &device_create_info, None)?;
    let graphics_queue = device.get_device_queue(graphics_queue_family, 0);

    println!("✅ Created logical device");

    // 5. Create swapchain
    let swapchain_loader = ash::khr::swapchain::Device::new(&instance, &device);

    let surface_capabilities = surface_loader
        .get_physical_device_surface_capabilities(physical_device, surface)?;

    let surface_formats = surface_loader
        .get_physical_device_surface_formats(physical_device, surface)?;

    let surface_format = surface_formats
        .iter()
        .find(|format| {
            format.format == vk::Format::B8G8R8A8_SRGB
                && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .unwrap_or(&surface_formats[0]);

    let swapchain_extent = vk::Extent2D { width, height };

    let image_count = (surface_capabilities.min_image_count + 1)
        .min(if surface_capabilities.max_image_count > 0 {
            surface_capabilities.max_image_count
        } else {
            u32::MAX
        });

    let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
        .surface(surface)
        .min_image_count(image_count)
        .image_format(surface_format.format)
        .image_color_space(surface_format.color_space)
        .image_extent(swapchain_extent)
        .image_array_layers(1)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .pre_transform(surface_capabilities.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(vk::PresentModeKHR::FIFO)
        .clipped(true);

    let swapchain = swapchain_loader.create_swapchain(&swapchain_create_info, None)?;
    let swapchain_images = swapchain_loader.get_swapchain_images(swapchain)?;

    println!("✅ Created swapchain with {} images", swapchain_images.len());

    // 6. Create image views
    let swapchain_image_views: Vec<_> = swapchain_images
        .iter()
        .map(|&image| {
            let create_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(surface_format.format)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });
            device.create_image_view(&create_info, None)
        })
        .collect::<Result<Vec<_>, _>>()?;

    println!("✅ Created image views");

    // 7. Create render pass
    let color_attachment = vk::AttachmentDescription::default()
        .format(surface_format.format)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

    let color_attachment_ref = vk::AttachmentReference::default()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    let subpass = vk::SubpassDescription::default()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(std::slice::from_ref(&color_attachment_ref));

    let dependency = vk::SubpassDependency::default()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .src_access_mask(vk::AccessFlags::empty())
        .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);

    let render_pass_create_info = vk::RenderPassCreateInfo::default()
        .attachments(std::slice::from_ref(&color_attachment))
        .subpasses(std::slice::from_ref(&subpass))
        .dependencies(std::slice::from_ref(&dependency));

    let render_pass = device.create_render_pass(&render_pass_create_info, None)?;

    println!("✅ Created render pass");

    // 8. Create descriptor set layout for texture sampling
    let sampler_binding = vk::DescriptorSetLayoutBinding::default()
        .binding(0)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT);

    let descriptor_set_layout_create_info = vk::DescriptorSetLayoutCreateInfo::default()
        .bindings(std::slice::from_ref(&sampler_binding));

    let descriptor_set_layout = device.create_descriptor_set_layout(&descriptor_set_layout_create_info, None)?;

    println!("✅ Created descriptor set layout");

    // 9. Create graphics pipeline
    let (graphics_pipeline, pipeline_layout) = create_graphics_pipeline(
        &device,
        render_pass,
        surface_format.format,
        swapchain_extent,
        descriptor_set_layout,
    )?;

    println!("✅ Created graphics pipeline");

    // 10. Create framebuffers
    let swapchain_framebuffers: Vec<_> = swapchain_image_views
        .iter()
        .map(|&image_view| {
            let attachments = [image_view];
            let framebuffer_create_info = vk::FramebufferCreateInfo::default()
                .render_pass(render_pass)
                .attachments(&attachments)
                .width(swapchain_extent.width)
                .height(swapchain_extent.height)
                .layers(1);
            device.create_framebuffer(&framebuffer_create_info, None)
        })
        .collect::<Result<Vec<_>, _>>()?;

    println!("✅ Created framebuffers");

    // 11. Create vertex buffer for fullscreen quad
    let vertices: [f32; 16] = [
        // pos (x, y)    tex (u, v)
        -1.0, -1.0,      0.0, 1.0,  // bottom-left
        -1.0,  1.0,      0.0, 0.0,  // top-left
         1.0, -1.0,      1.0, 1.0,  // bottom-right
         1.0,  1.0,      1.0, 0.0,  // top-right
    ];

    let buffer_size = std::mem::size_of_val(&vertices) as vk::DeviceSize;

    let buffer_create_info = vk::BufferCreateInfo::default()
        .size(buffer_size)
        .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let vertex_buffer = device.create_buffer(&buffer_create_info, None)?;

    let mem_requirements = device.get_buffer_memory_requirements(vertex_buffer);
    let memory_type_index = find_memory_type(
        &instance,
        physical_device,
        mem_requirements.memory_type_bits,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;

    let allocate_info = vk::MemoryAllocateInfo::default()
        .allocation_size(mem_requirements.size)
        .memory_type_index(memory_type_index);

    let vertex_buffer_memory = device.allocate_memory(&allocate_info, None)?;
    device.bind_buffer_memory(vertex_buffer, vertex_buffer_memory, 0)?;

    // Map memory and copy vertex data
    let data_ptr = device.map_memory(
        vertex_buffer_memory,
        0,
        buffer_size,
        vk::MemoryMapFlags::empty(),
    )?;
    std::ptr::copy_nonoverlapping(vertices.as_ptr(), data_ptr as *mut f32, vertices.len());
    device.unmap_memory(vertex_buffer_memory);

    println!("✅ Created vertex buffer");

    // 12. Create command pool
    let command_pool_create_info = vk::CommandPoolCreateInfo::default()
        .queue_family_index(graphics_queue_family)
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

    let command_pool = device.create_command_pool(&command_pool_create_info, None)?;

    // 13. Allocate command buffers
    let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::default()
        .command_pool(command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(swapchain_framebuffers.len() as u32);

    let command_buffers = device.allocate_command_buffers(&command_buffer_allocate_info)?;

    println!("✅ Created command pool and buffers");

    // 14. Create descriptor pool
    let pool_size = vk::DescriptorPoolSize::default()
        .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count(swapchain_images.len() as u32);

    let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::default()
        .pool_sizes(std::slice::from_ref(&pool_size))
        .max_sets(swapchain_images.len() as u32);

    let descriptor_pool = device.create_descriptor_pool(&descriptor_pool_create_info, None)?;

    // Allocate descriptor sets
    let layouts = vec![descriptor_set_layout; swapchain_images.len()];
    let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::default()
        .descriptor_pool(descriptor_pool)
        .set_layouts(&layouts);

    let descriptor_sets = device.allocate_descriptor_sets(&descriptor_set_allocate_info)?;

    println!("✅ Created descriptor pool and sets");

    // 15. Create sampler
    let sampler_create_info = vk::SamplerCreateInfo::default()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .anisotropy_enable(false)
        .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
        .unnormalized_coordinates(false)
        .compare_enable(false)
        .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
        .mip_lod_bias(0.0)
        .min_lod(0.0)
        .max_lod(0.0);

    let sampler = device.create_sampler(&sampler_create_info, None)?;

    println!("✅ Created sampler");

    // 16. Create synchronization objects
    let semaphore_create_info = vk::SemaphoreCreateInfo::default();
    let fence_create_info = vk::FenceCreateInfo::default()
        .flags(vk::FenceCreateFlags::SIGNALED);

    let image_available_semaphore = device.create_semaphore(&semaphore_create_info, None)?;
    let render_finished_semaphore = device.create_semaphore(&semaphore_create_info, None)?;
    let in_flight_fence = device.create_fence(&fence_create_info, None)?;

    println!("✅ Created synchronization objects");
    println!("🎉 Vulkan initialization complete!");

    Ok(VulkanState {
        entry,
        instance,
        physical_device,
        device,
        graphics_queue,
        graphics_queue_family,
        surface,
        surface_loader,
        swapchain: Some(swapchain),
        swapchain_loader,
        swapchain_images,
        swapchain_image_views,
        swapchain_framebuffers,
        render_pass,
        graphics_pipeline,
        pipeline_layout,
        vertex_buffer,
        vertex_buffer_memory,
        command_pool,
        command_buffers,
        descriptor_set_layout,
        descriptor_pool,
        descriptor_sets,
        gpui_texture: None,
        gpui_texture_view: None,
        gpui_texture_memory: None,
        sampler,
        image_available_semaphore,
        render_finished_semaphore,
        in_flight_fence,
        swapchain_format: surface_format.format,
        swapchain_extent,
        allocator: None,
    })
}

/// Create graphics pipeline with shaders
unsafe fn create_graphics_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    _format: vk::Format,
    extent: vk::Extent2D,
    descriptor_set_layout: vk::DescriptorSetLayout,
) -> Result<(vk::Pipeline, vk::PipelineLayout)> {
    // Load shaders
    let vert_shader_module = create_shader_module(device, VERT_SHADER_SPIRV)?;
    let frag_shader_module = create_shader_module(device, FRAG_SHADER_SPIRV)?;

    let entry_point = CStr::from_bytes_with_nul_unchecked(b"main\0");

    let vert_shader_stage = vk::PipelineShaderStageCreateInfo::default()
        .stage(vk::ShaderStageFlags::VERTEX)
        .module(vert_shader_module)
        .name(entry_point);

    let frag_shader_stage = vk::PipelineShaderStageCreateInfo::default()
        .stage(vk::ShaderStageFlags::FRAGMENT)
        .module(frag_shader_module)
        .name(entry_point);

    let shader_stages = [vert_shader_stage, frag_shader_stage];

    // Vertex input: position (vec2) + texcoord (vec2)
    let binding_description = vk::VertexInputBindingDescription::default()
        .binding(0)
        .stride(16) // 4 floats * 4 bytes
        .input_rate(vk::VertexInputRate::VERTEX);

    let attribute_descriptions = [
        vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(0),
        vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(8),
    ];

    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(std::slice::from_ref(&binding_description))
        .vertex_attribute_descriptions(&attribute_descriptions);

    let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_STRIP)
        .primitive_restart_enable(false);

    let viewport = vk::Viewport::default()
        .x(0.0)
        .y(0.0)
        .width(extent.width as f32)
        .height(extent.height as f32)
        .min_depth(0.0)
        .max_depth(1.0);

    let scissor = vk::Rect2D::default()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(extent);

    let viewport_state = vk::PipelineViewportStateCreateInfo::default()
        .viewports(std::slice::from_ref(&viewport))
        .scissors(std::slice::from_ref(&scissor));

    let rasterization_state = vk::PipelineRasterizationStateCreateInfo::default()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::BACK)
        .front_face(vk::FrontFace::CLOCKWISE)
        .depth_bias_enable(false);

    let multisample_state = vk::PipelineMultisampleStateCreateInfo::default()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

    // Alpha blending for GPUI layer
    let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
        .color_write_mask(vk::ColorComponentFlags::RGBA)
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
        .alpha_blend_op(vk::BlendOp::ADD);

    let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
        .logic_op_enable(false)
        .attachments(std::slice::from_ref(&color_blend_attachment));

    let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::default()
        .set_layouts(std::slice::from_ref(&descriptor_set_layout));

    let pipeline_layout = device.create_pipeline_layout(&pipeline_layout_create_info, None)?;

    let pipeline_create_info = vk::GraphicsPipelineCreateInfo::default()
        .stages(&shader_stages)
        .vertex_input_state(&vertex_input_state)
        .input_assembly_state(&input_assembly_state)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization_state)
        .multisample_state(&multisample_state)
        .color_blend_state(&color_blend_state)
        .layout(pipeline_layout)
        .render_pass(render_pass)
        .subpass(0);

    let pipelines = device.create_graphics_pipelines(
        vk::PipelineCache::null(),
        std::slice::from_ref(&pipeline_create_info),
        None,
    ).map_err(|(_, e)| e)?;

    // Clean up shader modules
    device.destroy_shader_module(vert_shader_module, None);
    device.destroy_shader_module(frag_shader_module, None);

    Ok((pipelines[0], pipeline_layout))
}

/// Create shader module from SPIR-V bytecode
unsafe fn create_shader_module(device: &ash::Device, spirv_code: &[u8]) -> Result<vk::ShaderModule> {
    let code = ash::util::read_spv(&mut std::io::Cursor::new(spirv_code))?;
    let create_info = vk::ShaderModuleCreateInfo::default().code(&code);
    device.create_shader_module(&create_info, None).map_err(Into::into)
}

/// Find suitable memory type index
unsafe fn find_memory_type(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    type_filter: u32,
    properties: vk::MemoryPropertyFlags,
) -> Result<u32> {
    let mem_properties = instance.get_physical_device_memory_properties(physical_device);

    for i in 0..mem_properties.memory_type_count {
        if (type_filter & (1 << i)) != 0
            && mem_properties.memory_types[i as usize].property_flags.contains(properties)
        {
            return Ok(i);
        }
    }

    anyhow::bail!("Failed to find suitable memory type")
}

/// Cleanup Vulkan resources
pub unsafe fn cleanup_vulkan(vk_state: &mut VulkanState) {
    let device = &vk_state.device;

    // Wait for device to be idle
    let _ = device.device_wait_idle();

    // Destroy synchronization objects
    device.destroy_fence(vk_state.in_flight_fence, None);
    device.destroy_semaphore(vk_state.render_finished_semaphore, None);
    device.destroy_semaphore(vk_state.image_available_semaphore, None);

    // Destroy sampler
    device.destroy_sampler(vk_state.sampler, None);

    // Destroy descriptor pool (automatically frees descriptor sets)
    device.destroy_descriptor_pool(vk_state.descriptor_pool, None);
    device.destroy_descriptor_set_layout(vk_state.descriptor_set_layout, None);

    // Destroy command pool (automatically frees command buffers)
    device.destroy_command_pool(vk_state.command_pool, None);

    // Destroy vertex buffer
    device.destroy_buffer(vk_state.vertex_buffer, None);
    device.free_memory(vk_state.vertex_buffer_memory, None);

    // Destroy pipeline and layout
    device.destroy_pipeline(vk_state.graphics_pipeline, None);
    device.destroy_pipeline_layout(vk_state.pipeline_layout, None);

    // Destroy framebuffers
    for &framebuffer in &vk_state.swapchain_framebuffers {
        device.destroy_framebuffer(framebuffer, None);
    }

    // Destroy render pass
    device.destroy_render_pass(vk_state.render_pass, None);

    // Destroy image views
    for &image_view in &vk_state.swapchain_image_views {
        device.destroy_image_view(image_view, None);
    }

    // Destroy swapchain
    if let Some(swapchain) = vk_state.swapchain {
        vk_state.swapchain_loader.destroy_swapchain(swapchain, None);
    }

    // Destroy GPUI texture if exists
    if let Some(texture_view) = vk_state.gpui_texture_view {
        device.destroy_image_view(texture_view, None);
    }
    if let Some(texture) = vk_state.gpui_texture {
        device.destroy_image(texture, None);
    }
    if let Some(memory) = vk_state.gpui_texture_memory {
        device.free_memory(memory, None);
    }

    // Destroy surface
    vk_state.surface_loader.destroy_surface(vk_state.surface, None);

    // Destroy device
    device.destroy_device(None);

    // Destroy instance
    vk_state.instance.destroy_instance(None);

    println!("✅ Cleaned up Vulkan resources");
}

/// Render a frame using Vulkan
pub unsafe fn render_frame(
    vk_state: &mut VulkanState,
    clear_color: [f32; 4],
) -> Result<()> {
    let device = &vk_state.device;

    // Wait for previous frame to finish
    device.wait_for_fences(
        &[vk_state.in_flight_fence],
        true,
        u64::MAX,
    )?;
    device.reset_fences(&[vk_state.in_flight_fence])?;

    // Acquire next swapchain image
    let swapchain = vk_state.swapchain.ok_or_else(|| anyhow::anyhow!("No swapchain"))?;

    let (image_index, _is_suboptimal) = vk_state.swapchain_loader.acquire_next_image(
        swapchain,
        u64::MAX,
        vk_state.image_available_semaphore,
        vk::Fence::null(),
    )?;

    // Reset and begin recording command buffer
    let command_buffer = vk_state.command_buffers[image_index as usize];
    device.reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?;

    let begin_info = vk::CommandBufferBeginInfo::default()
        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

    device.begin_command_buffer(command_buffer, &begin_info)?;

    // Begin render pass
    let clear_value = vk::ClearValue {
        color: vk::ClearColorValue {
            float32: clear_color,
        },
    };

    let render_pass_begin_info = vk::RenderPassBeginInfo::default()
        .render_pass(vk_state.render_pass)
        .framebuffer(vk_state.swapchain_framebuffers[image_index as usize])
        .render_area(vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk_state.swapchain_extent,
        })
        .clear_values(std::slice::from_ref(&clear_value));

    device.cmd_begin_render_pass(
        command_buffer,
        &render_pass_begin_info,
        vk::SubpassContents::INLINE,
    );

    // Bind pipeline
    device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        vk_state.graphics_pipeline,
    );

    // Bind vertex buffer
    device.cmd_bind_vertex_buffers(
        command_buffer,
        0,
        &[vk_state.vertex_buffer],
        &[0],
    );

    // If we have a GPUI texture, bind its descriptor set and draw
    if vk_state.gpui_texture_view.is_some() {
        device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            vk_state.pipeline_layout,
            0,
            &[vk_state.descriptor_sets[image_index as usize]],
            &[],
        );

        // Draw fullscreen quad (4 vertices, triangle strip)
        device.cmd_draw(command_buffer, 4, 1, 0, 0);
    }

    // End render pass
    device.cmd_end_render_pass(command_buffer);

    // End command buffer
    device.end_command_buffer(command_buffer)?;

    // Submit command buffer
    let wait_semaphores = [vk_state.image_available_semaphore];
    let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
    let signal_semaphores = [vk_state.render_finished_semaphore];
    let command_buffers = [command_buffer];

    let submit_info = vk::SubmitInfo::default()
        .wait_semaphores(&wait_semaphores)
        .wait_dst_stage_mask(&wait_stages)
        .command_buffers(&command_buffers)
        .signal_semaphores(&signal_semaphores);

    device.queue_submit(
        vk_state.graphics_queue,
        &[submit_info],
        vk_state.in_flight_fence,
    )?;

    // Present
    let swapchains = [swapchain];
    let image_indices = [image_index];

    let present_info = vk::PresentInfoKHR::default()
        .wait_semaphores(&signal_semaphores)
        .swapchains(&swapchains)
        .image_indices(&image_indices);

    vk_state.swapchain_loader.queue_present(vk_state.graphics_queue, &present_info)?;

    Ok(())
}

/// Recreate swapchain (for window resize)
pub unsafe fn recreate_swapchain(
    vk_state: &mut VulkanState,
    width: u32,
    height: u32,
) -> Result<()> {
    let device = &vk_state.device;

    // Wait for device to be idle
    device.device_wait_idle()?;

    // Clean up old swapchain resources
    for &framebuffer in &vk_state.swapchain_framebuffers {
        device.destroy_framebuffer(framebuffer, None);
    }
    vk_state.swapchain_framebuffers.clear();

    for &image_view in &vk_state.swapchain_image_views {
        device.destroy_image_view(image_view, None);
    }
    vk_state.swapchain_image_views.clear();

    if let Some(old_swapchain) = vk_state.swapchain.take() {
        vk_state.swapchain_loader.destroy_swapchain(old_swapchain, None);
    }

    // Get new surface capabilities
    let surface_capabilities = vk_state.surface_loader
        .get_physical_device_surface_capabilities(vk_state.physical_device, vk_state.surface)?;

    let swapchain_extent = vk::Extent2D { width, height };

    let image_count = (surface_capabilities.min_image_count + 1)
        .min(if surface_capabilities.max_image_count > 0 {
            surface_capabilities.max_image_count
        } else {
            u32::MAX
        });

    // Create new swapchain
    let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
        .surface(vk_state.surface)
        .min_image_count(image_count)
        .image_format(vk_state.swapchain_format)
        .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
        .image_extent(swapchain_extent)
        .image_array_layers(1)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .pre_transform(surface_capabilities.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(vk::PresentModeKHR::FIFO)
        .clipped(true);

    let swapchain = vk_state.swapchain_loader.create_swapchain(&swapchain_create_info, None)?;
    vk_state.swapchain = Some(swapchain);
    vk_state.swapchain_extent = swapchain_extent;

    // Get new swapchain images
    vk_state.swapchain_images = vk_state.swapchain_loader.get_swapchain_images(swapchain)?;

    // Recreate image views
    vk_state.swapchain_image_views = vk_state.swapchain_images
        .iter()
        .map(|&image| {
            let create_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(vk_state.swapchain_format)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });
            device.create_image_view(&create_info, None)
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Recreate framebuffers
    vk_state.swapchain_framebuffers = vk_state.swapchain_image_views
        .iter()
        .map(|&image_view| {
            let attachments = [image_view];
            let framebuffer_create_info = vk::FramebufferCreateInfo::default()
                .render_pass(vk_state.render_pass)
                .attachments(&attachments)
                .width(swapchain_extent.width)
                .height(swapchain_extent.height)
                .layers(1);
            device.create_framebuffer(&framebuffer_create_info, None)
        })
        .collect::<Result<Vec<_>, _>>()?;

    println!("✅ Recreated swapchain: {}x{}", width, height);

    Ok(())
}
