$path = "src/render/pipeline.rs"
$content = Get-Content $path

# Remove max_vertices field
$content = $content | Where-Object { $_ -notmatch 'pub max_vertices: usize,' }

# Remove blur_bg_layout field (the one in RenderPipeline struct)
# It's at line 82 approx.
$content = $content | Where-Object { $_ -notmatch 'pub blur_bg_layout: wgpu::BindGroupLayout,' }

# Rename cover_texture to _cover_texture in RenderPipeline struct
$content = $content -replace 'pub cover_texture: wgpu::Texture,', '    pub _cover_texture: wgpu::Texture,'

# Remove pending_ahb_release
$content = $content | Where-Object { $_ -notmatch 'pub pending_ahb_release: Option<\*mut std::ffi::c_void>,' }

# Replace max_vertices usage in vector_vertex_buffer creation
$content = $content -replace 'size: \(max_vertices \* std::mem::size_of::<Vertex>\(\)\) as u64,', '            size: (10000 * std::mem::size_of::<Vertex>()) as u64,'

# Update Self return block
$content = $content | Where-Object { $_ -notmatch 'max_vertices: 10000,' }
$content = $content -replace 'cover_texture,', '            _cover_texture: cover_texture,'
$content = $content | Where-Object { $_ -notmatch 'blur_bg_layout,' }
$content = $content | Where-Object { $_ -notmatch 'pending_ahb_release: None,' }

$content | Set-Content $path
