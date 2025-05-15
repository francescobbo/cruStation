
// A helper struct to bundle a texture, its view, and a sampler together.
// This is particularly useful for textures that will be rendered to and then sampled from.
pub struct TextureRenderTarget {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
}

impl TextureRenderTarget {
    // Creates a new TextureRenderTarget.
    //
    // Args:
    // - device: The WGPU device used to create GPU resources.
    // - width, height: Dimensions of the texture.
    // - format: The pixel format of the texture.
    // - usage: How the texture will be used (e.g., as a render target, for sampling).
    // - label: A debug label for the texture.
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        label: Option<&str>,
    ) -> Self {
        // Define the properties of the texture.
        let texture_descriptor = wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1, // 2D texture, so depth is 1.
            },
            mip_level_count: 1,     // No mipmaps for this example.
            sample_count: 1,        // No multisampling for this example.
            dimension: wgpu::TextureDimension::D2, // It's a 2D texture.
            format,                 // Use the specified format.
            usage,                  // Use the specified usage flags.
            view_formats: &[],      // Additional view formats, not needed here.
        };
        let texture = device.create_texture(&texture_descriptor);

        // A TextureView provides a way to access the texture's data (e.g., for rendering or sampling).
        // Using default settings for the view here.
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // A Sampler defines how a texture is sampled in a shader (e.g., filtering, wrapping).
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("{} Sampler", label.unwrap_or("Texture"))),
            address_mode_u: wgpu::AddressMode::ClampToEdge, // How to handle U coords outside [0,1].
            address_mode_v: wgpu::AddressMode::ClampToEdge, // How to handle V coords outside [0,1].
            address_mode_w: wgpu::AddressMode::ClampToEdge, // How to handle W coords outside [0,1].
            mag_filter: wgpu::FilterMode::Linear,     // Filtering when texture is magnified.
            min_filter: wgpu::FilterMode::Linear,     // Filtering when texture is minified.
            mipmap_filter: wgpu::FilterMode::Nearest, // Filtering between mipmap levels (not used here).
            ..Default::default() // Sensible defaults for other options.
        });

        Self {
            texture,
            view,
            sampler,
            width,
            height,
            format,
        }
    }
}