//! wgpu による描画。DrawList のインスタンスを 1 回のドローコールで描く。

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use std::collections::HashMap;

use crate::draw::{image_id, Color, DrawList, Instance};
use crate::image::RgbaImage;
use crate::text::{TextSystem, ATLAS_SIZE};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    screen: [f32; 2],
    scale: f32,
    srgb: f32,
}

pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    globals: wgpu::Buffer,
    instances: wgpu::Buffer,
    instance_capacity: usize,
    atlas: wgpu::Texture,
    srgb_target: bool,
    /// 読み込んだ画像（id → group 1 のバインドグループ）
    image_layout: wgpu::BindGroupLayout,
    image_sampler: wgpu::Sampler,
    images: HashMap<u32, wgpu::BindGroup>,
    /// 画像を描かないときに group 1 に置いておく 1×1 の透明な画像
    blank_image: wgpu::BindGroup,
}

impl Renderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        sprite_atlas_pngs: &[&[u8]],
    ) -> Result<Self, String> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("juku-render shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let globals = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("globals"),
            contents: bytemuck::bytes_of(&Globals { screen: [1.0, 1.0], scale: 1.0, srgb: 0.0 }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let atlas = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyph atlas"),
            size: wgpu::Extent3d { width: ATLAS_SIZE, height: ATLAS_SIZE, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let atlas_view = atlas.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("atlas sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let sprites = Self::load_sprites(device, queue, sprite_atlas_pngs)?;
        let sprite_view = sprites.create_view(&wgpu::TextureViewDescriptor::default());
        let sprite_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sprite sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("juku-render layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("juku-render bind group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: globals.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&atlas_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&sprite_view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&sprite_sampler) },
            ],
        });

        let image_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("image layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let image_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("image sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let blank = RgbaImage { width: 1, height: 1, pixels: vec![0; 4] };
        let blank_image = Self::make_image(device, queue, &image_layout, &image_sampler, &blank);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("juku-render pipeline layout"),
            bind_group_layouts: &[Some(&layout), Some(&image_layout)],
            immediate_size: 0,
        });

        const ATTRS: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![
            0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4,
            4 => Float32x4, 5 => Float32x4, 6 => Float32x4
        ];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("juku-render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Instance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &ATTRS,
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let instance_capacity = 4096;
        let instances = Self::make_instance_buffer(device, instance_capacity);

        Ok(Renderer {
            pipeline,
            bind_group,
            globals,
            instances,
            instance_capacity,
            atlas,
            srgb_target: format.is_srgb(),
            image_layout,
            image_sampler,
            images: HashMap::new(),
            blank_image,
        })
    }

    fn make_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        img: &RgbaImage,
    ) -> wgpu::BindGroup {
        let levels = img.mip_chain();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("image"),
            size: wgpu::Extent3d { width: img.width, height: img.height, depth_or_array_layers: 1 },
            mip_level_count: levels.len() as u32,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (level, l) in levels.iter().enumerate() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &l.pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(l.width * 4),
                    rows_per_image: Some(l.height),
                },
                wgpu::Extent3d { width: l.width, height: l.height, depth_or_array_layers: 1 },
            );
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("image bind group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
            ],
        })
    }

    /// 画像を GPU に送り、`DrawList::image` の `id` で描けるようにする（同じ id は差し替え）。
    /// 端末のテクスチャ上限を超える画像は縮める。
    pub fn set_image(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, id: u32, img: RgbaImage) {
        let limit = device.limits().max_texture_dimension_2d;
        let img = img.fit_within(limit, limit);
        let group = Self::make_image(device, queue, &self.image_layout, &self.image_sampler, &img);
        self.images.insert(id, group);
    }

    /// 画像を GPU から外す。
    pub fn remove_image(&mut self, id: u32) {
        self.images.remove(&id);
    }

    fn decode_sprite_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
        let img = crate::image::decode_png(bytes).map_err(|e| format!("sprite atlas: {e}"))?;
        Ok((img.width, img.height, img.pixels))
    }

    fn load_sprites(device: &wgpu::Device, queue: &wgpu::Queue, pngs: &[&[u8]]) -> Result<wgpu::Texture, String> {
        let mut pngs = pngs.iter();
        let (width, mut height, mut pixels) =
            if let Some(first) = pngs.next() { Self::decode_sprite_png(first)? } else { (1, 1, vec![0; 4]) };
        let limit = device.limits().max_texture_dimension_2d;
        if width.max(height) > limit {
            return Err("sprite atlas exceeds device texture limits".into());
        }
        for bytes in pngs {
            let (strip_width, strip_height, strip_pixels) = Self::decode_sprite_png(bytes)?;
            if strip_width != width {
                return Err("sprite atlas PNGs must have the same width".into());
            }
            height = height
                .checked_add(strip_height)
                .filter(|&height| height <= limit)
                .ok_or("sprite atlas exceeds device texture limits")?;
            pixels.extend(strip_pixels);
        }
        Ok(device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("sprite atlas"),
                size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &pixels,
        ))
    }

    fn make_instance_buffer(device: &wgpu::Device, cap: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instances"),
            size: (cap * std::mem::size_of::<Instance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// `target` に描く。`size_px` は物理ピクセル、`scale` は論理→物理の倍率。
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        size_px: (u32, u32),
        scale: f32,
        list: &DrawList,
        text: &mut TextSystem,
        clear: Color,
    ) {
        if let Some(pixels) = text.atlas_pixels() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.atlas,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(ATLAS_SIZE),
                    rows_per_image: Some(ATLAS_SIZE),
                },
                wgpu::Extent3d { width: ATLAS_SIZE, height: ATLAS_SIZE, depth_or_array_layers: 1 },
            );
        }

        let g = Globals {
            screen: [size_px.0 as f32 / scale, size_px.1 as f32 / scale],
            scale,
            srgb: if self.srgb_target { 1.0 } else { 0.0 },
        };
        queue.write_buffer(&self.globals, 0, bytemuck::bytes_of(&g));

        let n = list.instances.len();
        if n > self.instance_capacity {
            self.instance_capacity = n.next_power_of_two();
            self.instances = Self::make_instance_buffer(device, self.instance_capacity);
        }
        if n > 0 {
            queue.write_buffer(&self.instances, 0, bytemuck::cast_slice(&list.instances));
        }

        let mut clear_c = clear;
        if self.srgb_target {
            let lin = |c: f32| c.powf(2.2);
            clear_c = Color { r: lin(clear.r), g: lin(clear.g), b: lin(clear.b), a: clear.a };
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear_c.r as f64,
                            g: clear_c.g as f64,
                            b: clear_c.b as f64,
                            a: clear_c.a as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if n > 0 {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                // 画像が変わるところでドローコールを分ける（ほかの図形は group 1 を使わない）
                let mut bound: Option<u32> = None;
                pass.set_bind_group(1, &self.blank_image, &[]);
                let mut start = 0;
                for (i, inst) in list.instances.iter().enumerate() {
                    let Some(id) = image_id(inst) else { continue };
                    if bound == Some(id) {
                        continue;
                    }
                    if i > start {
                        draw_range(&mut pass, &self.instances, start, i);
                    }
                    pass.set_bind_group(1, self.images.get(&id).unwrap_or(&self.blank_image), &[]);
                    bound = Some(id);
                    start = i;
                }
                draw_range(&mut pass, &self.instances, start, n);
            }
        }
        queue.submit(std::iter::once(encoder.finish()));
    }
}

/// インスタンス `start..end` を描く。WebGL2 は先頭インスタンスの指定に対応しないので、
/// 頂点バッファの開始位置をずらして 0 番から描く。
fn draw_range(pass: &mut wgpu::RenderPass<'_>, instances: &wgpu::Buffer, start: usize, end: usize) {
    let size = std::mem::size_of::<Instance>() as u64;
    pass.set_vertex_buffer(0, instances.slice(start as u64 * size..end as u64 * size));
    pass.draw(0..6, 0..(end - start) as u32);
}
