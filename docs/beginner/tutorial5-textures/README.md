# Texturas e Bind Groups

## Carregando uma Imagem a partir de um Arquivo

Para aplicar uma imagem sobre a malha de triângulos, usamos **texturas** (diffuse maps ou texturas de cor).

Usamos a crate `image` para decodificar os bytes da imagem (ex: PNG ou JPEG) e enviá-los para a GPU:

```toml
[dependencies.image]
version = "0.24"
default-features = false
features = ["png", "jpeg"]
```

Carregando os dados e criando a textura em Rust (`wgpu::Texture`):

```rust
let diffuse_bytes = include_bytes!("happy-tree.png");
let diffuse_image = image::load_from_memory(diffuse_bytes).unwrap();
let diffuse_rgba = diffuse_image.to_rgba8();

use image::GenericImageView;
let dimensions = diffuse_image.dimensions();

let texture_size = wgpu::Extent3d {
    width: dimensions.0,
    height: dimensions.1,
    depth_or_array_layers: 1,
};
let diffuse_texture = device.create_texture(
    &wgpu::TextureDescriptor {
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        label: Some("diffuse_texture"),
        view_formats: &[],
    }
);
```

Enviando os pixels para a GPU usando a `queue`:

```rust
queue.write_texture(
    wgpu::TexelCopyTextureInfo {
        texture: &diffuse_texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
    },
    &diffuse_rgba,
    wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(4 * dimensions.0),
        rows_per_image: Some(dimensions.1),
    },
    texture_size,
);
```

## TextureViews e Samplers

Uma `TextureView` oferece uma visão dos dados da textura, enquanto o `Sampler` controla como a textura é amostrada (filtro bilinear/trilinear, repetição nas bordas `AddressMode`, etc.).

```rust
let diffuse_texture_view = diffuse_texture.create_view(&wgpu::TextureViewDescriptor::default());
let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
    address_mode_u: wgpu::AddressMode::ClampToEdge,
    address_mode_v: wgpu::AddressMode::ClampToEdge,
    address_mode_w: wgpu::AddressMode::ClampToEdge,
    mag_filter: wgpu::FilterMode::Linear,
    min_filter: wgpu::FilterMode::Nearest,
    mipmap_filter: wgpu::FilterMode::Nearest,
    ..Default::default()
});
```

## Bind Group e Bind Group Layout

O `BindGroup` conecta os recursos da GPU (como a `TextureView` e o `Sampler`) às posições esperadas nos shaders (`@group(0) @binding(0)`):

```rust
let texture_bind_group_layout =
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
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
        label: Some("texture_bind_group_layout"),
    });

let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    layout: &texture_bind_group_layout,
    entries: &[
        wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
        },
        wgpu::BindGroupEntry {
            binding: 1,
            resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
        },
    ],
    label: Some("diffuse_bind_group"),
});
```

Passamos o `texture_bind_group_layout` ao criar a pipeline (`PipelineLayout Descriptor`) e chamamos `set_bind_group` durante a render pass:

```rust
render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
```

## Shader com Amostragem de Textura

No shader WGSL:

```wgsl
@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}
```

![Textura aplicada sobre o quadrado](./happy-tree.png)

## Demonstração

<WasmExample example="tutorial5_textures"></WasmExample>

<AutoGithubLink/>
