$path = "src/render/pipeline.rs"
$content = Get-Content $path
$start = 712 # Line 713 (0-based)
$end = 760   # Line 761 (0-based)

$newLines = @(
    "            // Recreate main bind group with correct 4 entries (0-3)",
    "            self.bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {",
    "                layout: &self",
    "                    .device",
    "                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {",
    "                        entries: &[",
    "                            wgpu::BindGroupLayoutEntry {",
    "                                binding: 0,",
    "                                visibility: wgpu::ShaderStages::FRAGMENT,",
    "                                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),",
    "                                count: None,",
    "                            },",
    "                            wgpu::BindGroupLayoutEntry {",
    "                                binding: 1,",
    "                                visibility: wgpu::ShaderStages::FRAGMENT,",
    "                                ty: wgpu::BindingType::Texture {",
    "                                    sample_type: wgpu::TextureSampleType::Float {",
    "                                        filterable: true,",
    "                                    },",
    "                                    view_dimension: wgpu::TextureViewDimension::D2,",
    "                                    multisampled: false,",
    "                                },",
    "                                count: None,",
    "                            },",
    "                            wgpu::BindGroupLayoutEntry {",
    "                                binding: 2,",
    "                                visibility: wgpu::ShaderStages::FRAGMENT,",
    "                                ty: wgpu::BindingType::Texture {",
    "                                    sample_type: wgpu::TextureSampleType::Float {",
    "                                        filterable: true,",
    "                                    },",
    "                                    view_dimension: wgpu::TextureViewDimension::D2,",
    "                                    multisampled: false,",
    "                                },",
    "                                count: None,",
    "                            },",
    "                            wgpu::BindGroupLayoutEntry {",
    "                                binding: 3,",
    "                                visibility: wgpu::ShaderStages::FRAGMENT,",
    "                                ty: wgpu::BindingType::Texture {",
    "                                    sample_type: wgpu::TextureSampleType::Float {",
    "                                        filterable: true,",
    "                                    },",
    "                                    view_dimension: wgpu::TextureViewDimension::D2,",
    "                                    multisampled: false,",
    "                                },",
    "                                count: None,",
    "                            },"
)

$prefix = $content[0..($start-1)]
$suffix = $content[($end+1)..($content.Count-1)]

$newContent = $prefix + $newLines + $suffix
$newContent | Set-Content $path
