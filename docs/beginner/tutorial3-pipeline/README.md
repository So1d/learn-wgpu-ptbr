# A Pipeline (Pipeline de Renderização)

## O que é uma pipeline?

Se você já conhece OpenGL, lembrará de usar programas de shader. Uma pipeline é uma versão mais estruturada e robusta disso. A pipeline descreve todas as ações e etapas que a GPU executará ao processar um conjunto de dados. Nesta seção, criaremos um `RenderPipeline`.

## O que são Shaders?

Shaders são pequenos programas enviados à GPU para realizar operações com os seus dados. Existem três tipos principais de shaders: de vértice (vertex shader), de fragmento (fragment shader) e de computação (compute shader).

## Vértice, fragmento... o que são?

Um **vértice** é um ponto no espaço 3D (ou 2D). Os vértices são agrupados em triângulos, que compõem todas as formas geométricas nos gráficos 3D.

O **vertex shader** manipula as posições dos vértices para transformar as formas.

Os vértices são então convertidos em **fragmentos** (pixels potenciais na tela), e o **fragment shader** calcula a cor final de cada fragmento.

## WGSL

O [WebGPU Shading Language](https://www.w3.org/TR/WGSL/) (WGSL) é a linguagem padrão de shaders do WebGPU. O Wgpu compila WGSL internamente para o formato nativo da sua GPU (SPIR-V para Vulkan, MSL para Metal, HLSL para DirectX 12).

## Escrevendo os Shaders

Crie um arquivo `shader.wgsl` no mesmo diretório do seu código:

```wgsl
// Vertex shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(1 - i32(in_vertex_index)) * 0.5;
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.3, 0.2, 0.1, 1.0);
}
```

## Criando a Render Pipeline em Rust

Adicionamos a pipeline ao `State`:

```rust
// lib.rs
pub struct State {
    // ...
    render_pipeline: wgpu::RenderPipeline,
}
```

No método `new()`, carregamos o módulo de shader e definimos o layout da pipeline:

```rust
let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: Some("Shader"),
    source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
});

let render_pipeline_layout =
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });

let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("Render Pipeline"),
    layout: Some(&render_pipeline_layout),
    vertex: wgpu::VertexState {
        module: &shader,
        entry_point: Some("vs_main"),
        buffers: &[],
        compilation_options: wgpu::PipelineCompilationOptions::default(),
    },
    fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: Some("fs_main"),
        targets: &[Some(wgpu::ColorTargetState {
            format: config.format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        })],
        compilation_options: wgpu::PipelineCompilationOptions::default(),
    }),
    primitive: wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: Some(wgpu::Face::Back),
        polygon_mode: wgpu::PolygonMode::Fill,
        unclipped_depth: false,
        conservative: false,
    },
    depth_stencil: None,
    multisample: wgpu::MultisampleState {
        count: 1,
        mask: !0,
        alpha_to_coverage_enabled: false,
    },
    multiview_mask: None,
    cache: None,
});
```

## Executando a Render Pipeline

No método `render()`, ativamos a pipeline e emitimos a chamada de desenho:

```rust
    render_pass.set_pipeline(&self.render_pipeline);
    render_pass.draw(0..3, 0..1);
```

Com isso, o triângulo marrom é desenhado na tela!

![Triângulo renderizado](./tutorial3-pipeline-triangle.png)

## Demonstração

<WasmExample example="tutorial3_pipeline"></WasmExample>

<AutoGithubLink/>

## Desafio

Crie uma segunda pipeline usando as posições do triângulo para gerar cores dinâmicas e alterne entre as duas pipelines ao pressionar a barra de espaço.
