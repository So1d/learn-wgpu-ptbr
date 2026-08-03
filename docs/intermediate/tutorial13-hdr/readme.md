# High Dynamic Range Rendering (HDR)

## O que é High Dynamic Range (HDR)?

No render sRGB padrão de 8 bits por canal (256 níveis por cor), valores de brilho acima de `1.0` são cortados (*clipping*). Texturas HDR utilizam formatos de ponto flutuante de 16-bit (`wgpu::TextureFormat::Rgba16Float`), permitindo armazenar faixas de iluminação muito mais amplas e realistas.

## Mudando para HDR e Tonemapping

Renderizamos a cena em um target HDR (`Rgba16Float`) e, em seguida, aplicamos uma técnica de **Tone Mapping** (como Reinhard ou Aces) para converter os valores HDR para a superfície sRGB padrão do monitor.

### Estrutura em Rust (`hdr.rs`):

```rust
pub struct HdrPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    texture: texture::Texture,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    layout: wgpu::BindGroupLayout,
}
```

Usamos `Rgba16Float` como formato do buffer intermediário da cena.

### Shader de Tone Mapping (`hdr.wgsl`):

```wgsl
@group(0) @binding(0) var hdr_texture: texture_2d<f32>;
@group(0) @binding(1) var hdr_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let hdr_color = textureSample(hdr_texture, hdr_sampler, in.tex_coords).rgb;
    // Tone mapping de Reinhard: result = hdr / (hdr + 1.0)
    let mapped = hdr_color / (hdr_color + vec3<f32>(1.0));
    // Correção gama para sRGB
    let gamma_corrected = pow(mapped, vec3<f32>(1.0 / 2.2));
    return vec4<f32>(gamma_corrected, 1.0);
}
```

![Antes e depois da aplicação do HDR e Tonemapping](./after-hdr.png)

## Demonstração

<WasmExample example="tutorial13_hdr"></WasmExample>

<AutoGithubLink/>
