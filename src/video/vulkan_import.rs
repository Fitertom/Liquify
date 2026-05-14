use ash::vk;
use wgpu_hal as hal;

// ============================================================================
// Вспомогательные структуры
// ============================================================================

#[derive(Debug)]
pub struct AhbProps {
    pub allocation_size: u64,
    pub memory_type_bits: u32,
    pub external_format: u64,
    pub vk_format: vk::Format,
}

#[derive(Debug, Clone)]
pub struct AhbFormatProps {
    pub external_format: u64,
    pub suggested_ycbcr_model: vk::SamplerYcbcrModelConversion,
    pub suggested_ycbcr_range: vk::SamplerYcbcrRange,
    pub suggested_x_chroma_offset: vk::ChromaLocation,
    pub suggested_y_chroma_offset: vk::ChromaLocation,
    pub sampler_ycbcr_conversion_components: vk::ComponentMapping,
    pub format: vk::Format,
}

// ============================================================================
// dlsym-загрузчик vkGetAndroidHardwareBufferPropertiesANDROID
// ============================================================================

type FnGetAhbProps = extern "system" fn(
    vk::Device,
    *const std::ffi::c_void,
    *mut vk::AndroidHardwareBufferPropertiesANDROID,
) -> vk::Result;

unsafe fn load_get_ahb_props_fn() -> Option<FnGetAhbProps> {
    let lib = unsafe {
        libc::dlopen(
            c"libvulkan.so".as_ptr(),
            libc::RTLD_NOW | libc::RTLD_NOLOAD,
        )
    };
    if lib.is_null() {
        log::error!("vulkan_import: dlopen libvulkan.so failed");
        return None;
    }
    let sym = unsafe {
        libc::dlsym(lib, c"vkGetAndroidHardwareBufferPropertiesANDROID".as_ptr())
    };
    unsafe { libc::dlclose(lib) };
    if sym.is_null() {
        log::error!("vulkan_import: dlsym vkGetAndroidHardwareBufferPropertiesANDROID returned null");
        return None;
    }
    Some(unsafe { std::mem::transmute(sym) })
}

// ============================================================================
// Запрос свойств AHB — внутренняя реализация с фиксом E0503
// ============================================================================

unsafe fn query_ahb_props_internal(
    raw_device: &ash::Device,
    fn_get: FnGetAhbProps,
    buffer_ptr: *const std::ffi::c_void,
) -> Option<(AhbProps, AhbFormatProps)> {
    // ЕДИНСТВЕННЫЙ вызов fn_get — читаем props и format_props за один раз
    let mut format_props = vk::AndroidHardwareBufferFormatPropertiesANDROID::default();
    let mut props = vk::AndroidHardwareBufferPropertiesANDROID::default()
        .push_next(&mut format_props);

    let result = fn_get(raw_device.handle(), buffer_ptr, &mut props);

    if result != vk::Result::SUCCESS {
        log::error!("vkGetAndroidHardwareBufferPropertiesANDROID failed: {:?}", result);
        return None;
    }

    // Копируем ВСЕ нужные значения ДО того как format_props "заимствован" через props
    let allocation_size = props.allocation_size;
    let memory_type_bits = props.memory_type_bits;
    let external_format = format_props.external_format;
    let suggested_ycbcr_model = format_props.suggested_ycbcr_model;
    let suggested_ycbcr_range = format_props.suggested_ycbcr_range;
    let suggested_x_chroma_offset = format_props.suggested_x_chroma_offset;
    let suggested_y_chroma_offset = format_props.suggested_y_chroma_offset;
    let sampler_ycbcr_conversion_components = format_props.sampler_ycbcr_conversion_components;
    let vk_format = format_props.format;

    log::info!(
        "AHB props: alloc_size={}, memory_type_bits={:#x}, external_format={}, vk_format={:?}",
        allocation_size, memory_type_bits, external_format, vk_format,
    );

    Some((
        AhbProps {
            allocation_size,
            memory_type_bits,
            external_format,
            vk_format,
        },
        AhbFormatProps {
            external_format,
            suggested_ycbcr_model,
            suggested_ycbcr_range,
            suggested_x_chroma_offset,
            suggested_y_chroma_offset,
            sampler_ycbcr_conversion_components,
            format: vk_format,
        },
    ))
}

// ============================================================================
// Публичные API функции
// ============================================================================

/// Возвращает (external_format, AhbFormatProps) для инициализации YCbCr пайплайна
pub unsafe fn query_ahb_properties(
    raw_device: &ash::Device,
    buffer_ptr: *mut std::ffi::c_void,
) -> Option<(u64, AhbFormatProps)> {
    let fn_get = load_get_ahb_props_fn()?;
    let (ahb_props, format_props) = query_ahb_props_internal(
        raw_device, fn_get, buffer_ptr as *const _
    )?;
    Some((ahb_props.external_format, format_props))
}

/// Возвращает (external_format, AhbProps) для обработки каждого кадра
pub unsafe fn query_ahb_properties_struct(
    raw_device: &ash::Device,
    buffer_ptr: *mut std::ffi::c_void,
) -> Option<(u64, AhbProps)> {
    let fn_get = load_get_ahb_props_fn()?;
    let (ahb_props, _) = query_ahb_props_internal(
        raw_device, fn_get, buffer_ptr as *const _
    )?;
    Some((ahb_props.external_format, ahb_props))
}

// ============================================================================
// Импорт AHB в wgpu::Texture (stub, не используется в YCbCr пути)
// ============================================================================

fn pick_memory_type(memory_type_bits: u32) -> u32 {
    assert_ne!(memory_type_bits, 0);
    memory_type_bits.trailing_zeros()
}

/// Импортирует AHardwareBuffer* в wgpu-текстуру.
/// НЕ ИСПОЛЬЗУЕТСЯ при YCbCr convolution path — оставлен для совместимости.
pub unsafe fn import_android_buffer(
    device: &wgpu::Device,
    _queue: &wgpu::Queue,
    buffer_ptr: *mut std::ffi::c_void,
    width: u32,
    height: u32,
) -> Option<wgpu::Texture> {
    // Получаем HAL-устройство Vulkan
    let hal_device = device.as_hal::<hal::api::Vulkan>()?;
    let raw_device = hal_device.raw_device();
    
    // Загружаем функцию расширения
    let fn_get = unsafe { load_get_ahb_props_fn()? };
    
    // Запрашиваем свойства AHB
    let (ahb_props, _) = query_ahb_props_internal(
        raw_device, fn_get, buffer_ptr as *const _
    )?;
    
    // Создаём VkImage для AHB
    let mut external_image_info = vk::ExternalMemoryImageCreateInfo::default()
        .handle_types(vk::ExternalMemoryHandleTypeFlags::ANDROID_HARDWARE_BUFFER_ANDROID);
    
    let vk_image = if ahb_props.external_format != 0 {
        let mut external_format_info = vk::ExternalFormatANDROID::default()
            .external_format(ahb_props.external_format);
        
        let info = vk::ImageCreateInfo::default()
            .push_next(&mut external_image_info)
            .push_next(&mut external_format_info)
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::UNDEFINED)
            .extent(vk::Extent3D { width, height, depth: 1 })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);
        
        unsafe { raw_device.create_image(&info, None) }
    } else {
        let info = vk::ImageCreateInfo::default()
            .push_next(&mut external_image_info)
            .image_type(vk::ImageType::TYPE_2D)
            .format(ahb_props.vk_format)
            .extent(vk::Extent3D { width, height, depth: 1 })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);
        
        unsafe { raw_device.create_image(&info, None) }
    }
    .map_err(|e| log::error!("create_image failed: {:?}", e))
    .ok()?;
    
    // Импортируем память AHB
    let mut import_info = vk::ImportAndroidHardwareBufferInfoANDROID::default()
        .buffer(buffer_ptr as _);
    let mut dedicated_info = vk::MemoryDedicatedAllocateInfo::default()
        .image(vk_image);
    
    let memory_type_index = pick_memory_type(ahb_props.memory_type_bits);
    
    let alloc_info = vk::MemoryAllocateInfo::default()
        .allocation_size(ahb_props.allocation_size)
        .memory_type_index(memory_type_index)
        .push_next(&mut dedicated_info)
        .push_next(&mut import_info);
    
    let memory = unsafe { raw_device.allocate_memory(&alloc_info, None) }
        .map_err(|e| {
            log::error!("allocate_memory failed: {:?}", e);
            unsafe { raw_device.destroy_image(vk_image, None) };
        })
        .ok()?;
    
    if let Err(e) = unsafe { raw_device.bind_image_memory(vk_image, memory, 0) } {
        unsafe {
            raw_device.free_memory(memory, None);
            raw_device.destroy_image(vk_image, None);
        }
        log::error!("bind_image_memory failed: {:?}", e);
        return None;
    }
    
    // Определяем wgpu формат
    let wgpu_format = match ahb_props.vk_format {
        vk::Format::UNDEFINED => wgpu::TextureFormat::Rgba8Unorm,
        vk::Format::R8G8B8A8_UNORM => wgpu::TextureFormat::Rgba8Unorm,
        vk::Format::R8G8B8A8_SRGB => wgpu::TextureFormat::Rgba8UnormSrgb,
        vk::Format::B8G8R8A8_UNORM => wgpu::TextureFormat::Bgra8Unorm,
        vk::Format::B8G8R8A8_SRGB => wgpu::TextureFormat::Bgra8UnormSrgb,
        _ => wgpu::TextureFormat::Rgba8Unorm,
    };
    
    // Оборачиваем в HAL текстуру
    let hal_desc = hal::TextureDescriptor {
        label: Some("Android Video Texture"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu_format,
        usage: wgpu::TextureUses::RESOURCE,
        memory_flags: hal::MemoryFlags::empty(),
        view_formats: vec![],
    };
    
    let hal_texture = hal_device.texture_from_raw(
        vk_image,
        &hal_desc,
        None,
        hal::vulkan::TextureMemory::Dedicated(memory),
    );
    
    let texture_desc = wgpu::TextureDescriptor {
        label: Some("Android Video Texture"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu_format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    };
    
    let texture = unsafe {
        device.create_texture_from_hal::<hal::api::Vulkan>(hal_texture, &texture_desc)
    };
    Some(texture)
}
