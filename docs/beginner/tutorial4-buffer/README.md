# Buffers e Índices

## O que é um Buffer?

Um buffer é um bloco de dados armazenado na memória da GPU de forma contínua (sequencial). Buffers são usados para armazenar dados como estruturas de vértices, matrizes ou listas de índices.

## O Vertex Buffer (Buffer de Vértices)

Em vez de embutir os vértices diretamente no código do shader WGSL, enviamos uma estrutura de vértices da CPU para a GPU através de um `wgpu::Buffer`.

Definindo a estrutura `Vertex`:

```rust
// lib.rs
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}
```

Dados do triângulo:

```rust
const VERTICES: &[Vertex] = &[
    Vertex { position: [0.0, 0.5, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [-0.5, -0.5, 0.0], color: [0.0, 1.0, 0.0] },
    Vertex { position: [0.5, -0.5, 0.0], color: [0.0, 0.0, 1.0] },
];
```

Criando o buffer em `State::new()`:

```rust
let vertex_buffer = device.create_buffer_init(
    &wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(VERTICES),
        usage: wgpu::BufferUsages::VERTEX,
    }
);
```

Definindo o layout dos vértices (`VertexBufferLayout`):

```rust
impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                }
            ]
        }
    }
}
```

Vinculando o buffer no método `render()`:

```rust
render_pass.set_pipeline(&self.render_pipeline);
render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
render_pass.draw(0..self.num_vertices, 0..1);
```

## O Index Buffer (Buffer de Índices)

Quando desenhamos formas complexas, muitos vértices se repete entre triângulos adjacentes. Um **Index Buffer** armazena os índices desses vértices, economizando muita memória na GPU.

Exemplo de um pentágono com 5 vértices e 3 triângulos:

```rust
const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.0868241, 0.49240386, 0.0], color: [0.5, 0.0, 0.5] }, // A
    Vertex { position: [-0.49513406, 0.06958647, 0.0], color: [0.5, 0.0, 0.5] }, // B
    Vertex { position: [-0.21918549, -0.44939706, 0.0], color: [0.5, 0.0, 0.5] }, // C
    Vertex { position: [0.35966998, -0.3473291, 0.0], color: [0.5, 0.0, 0.5] }, // D
    Vertex { position: [0.44147372, 0.2347359, 0.0], color: [0.5, 0.0, 0.5] }, // E
];

const INDICES: &[u16] = &[
    0, 1, 4,
    1, 2, 4,
    2, 3, 4,
];
```

Criando o buffer de índices:

```rust
let index_buffer = device.create_buffer_init(
    &wgpu::util::BufferInitDescriptor {
        label: Some("Index Buffer"),
        contents: bytemuck::cast_slice(INDICES),
        usage: wgpu::BufferUsages::INDEX,
    }
);
let num_indices = INDICES.len() as u32;
```

E no `render()` usamos `set_index_buffer` e `draw_indexed`:

```rust
render_pass.set_pipeline(&self.render_pipeline);
render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
```

![Pentágono magenta](./indexed-pentagon.png)

## Demonstração

<WasmExample example="tutorial4_buffer"></WasmExample>

<AutoGithubLink/>

## Desafio

Crie uma forma geométrica mais complexa utilizando um buffer de vértices e um buffer de índices. Alterne entre os modelos ao pressionar a barra de espaço.
