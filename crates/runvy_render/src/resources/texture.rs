use runvy_asset::Handle;
use runvy_asset::TextureAsset;
use std::collections::HashMap;
use std::sync::Arc;

pub struct GpuTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

pub type GpuTextureHandle = Arc<GpuTexture>;

impl GpuTexture {
    pub fn from_asset(device: &wgpu::Device, queue: &wgpu::Queue, asset: &TextureAsset) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("GPU Texture"),
            size: wgpu::Extent3d {
                width: asset.width,
                height: asset.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // println!("📤 Uploading texture...");

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &asset.pixels, // ⚠️ см. ниже
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * asset.width),
                rows_per_image: Some(asset.height),
            },
            wgpu::Extent3d {
                width: asset.width,
                height: asset.height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
        }
    }
}

pub struct TextureCache {
    map: HashMap<usize, GpuTexture>,
}

impl TextureCache {
    pub fn get_or_create(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        handle: &Handle<TextureAsset>,
    ) -> &GpuTexture {
        let key = handle.inner.as_ref() as *const _ as usize;

        self.map
            .entry(key)
            .or_insert_with(|| GpuTexture::from_asset(device, queue, &handle.inner))
    }
}
