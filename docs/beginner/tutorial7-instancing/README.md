# Instanciamento

Nossa cena no momento é muito simples: temos um objeto centralizado em (0,0,0). E se quiséssemos mais objetos? É aqui que entra o instanciamento (instancing).

O instanciamento nos permite desenhar o mesmo objeto múltiplas vezes com propriedades diferentes (posição, orientação, tamanho, cor, etc.). Existem várias maneiras de fazer o instanciamento. Uma maneira seria modificar o uniform buffer para incluir essas propriedades e então atualizá-lo antes de desenhar cada instância do nosso objeto.

Não queremos usar esse método por razões de desempenho. Atualizar o uniform buffer para cada instância exigiria múltiplas cópias de buffer para cada frame. Além disso, nosso método para atualizar o uniform buffer atualmente exige que criemos um novo buffer para armazenar os dados atualizados. Isso é muito tempo desperdiçado entre chamadas de desenho (draw calls).

Se olharmos para os parâmetros da função `draw_indexed` [na documentação do wgpu](https://docs.rs/wgpu/latest/wgpu/struct.RenderPass.html#method.draw_indexed), podemos ver uma solução para o nosso problema.

```rust
pub fn draw_indexed(
    &mut self,
    indices: Range<u32>,
    base_vertex: i32,
    instances: Range<u32> // <-- Este aqui
)
```

O parâmetro `instances` recebe um `Range<u32>`. Esse parâmetro diz à GPU quantas cópias, ou instâncias, do modelo queremos desenhar. Atualmente, estamos especificando `0..1`, o que instrui a GPU a desenhar nosso modelo uma vez e depois parar. Se usássemos `0..5`, nosso código desenharia cinco instâncias.

O fato de `instances` ser um `Range<u32>` pode parecer estranho, já que usar `1..2` para instâncias ainda desenharia uma instância do nosso objeto. Parece que seria mais simples apenas usar um `u32`, certo? O motivo de ser um intervalo é que às vezes não queremos desenhar **todas** as nossas instâncias. Às vezes, queremos desenhar uma seleção delas porque outras não estão no enquadramento, ou estamos depurando e queremos olhar para um conjunto específico de instâncias.

Ok, agora sabemos como desenhar múltiplas instâncias de um objeto. Como dizemos ao wgpu qual instância específica desenhar? Vamos usar algo conhecido como instance buffer.

## O Instance Buffer

Criaremos um instance buffer de forma semelhante a como criamos um uniform buffer. Primeiro, criaremos uma struct chamada `Instance`.

```rust
// lib.rs
// ...

// NOVO!
struct Instance {
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
}
```

<Note>

Um `Quaternion` (quatérnio) é uma estrutura matemática frequentemente usada para representar rotações. A matemática por trás deles é complexa (envolve números imaginários e espaço 4D), então não vou abordá-la aqui. Se você realmente quiser se aprofundar neles [aqui está um artigo do Wolfram Alpha](https://mathworld.wolfram.com/Quaternion.html).

</Note>

Usar esses valores diretamente no shader seria trabalhoso, pois quatérnios não possuem um análogo em WGSL. Não quero escrever a matemática no shader, então vamos converter os dados de `Instance` em uma matriz e armazená-los em uma struct chamada `InstanceRaw`.

```rust
// NOVO!
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    model: [[f32; 4]; 4],
}
```

Estes são os dados que irão para o `wgpu::Buffer`. Mantemos estes separados para que possamos atualizar a `Instance` o quanto quisermos sem precisar mexer com matrizes. Só precisamos atualizar os dados brutos antes de desenhar.

Vamos criar um método na `Instance` para converter para `InstanceRaw`.

```rust
// NOVO!
impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (cgmath::Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation)).into(),
        }
    }
}
```

Agora precisamos adicionar dois campos ao `State`: `instances` e `instance_buffer`.

```rust
pub struct State {
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
}
```

A crate `cgmath` usa traits para fornecer métodos matemáticos comuns em suas structs, como `Vector3`, que devem ser importadas antes que esses métodos possam ser chamados. Por conveniência, o módulo `prelude` dentro da crate fornece as traits de extensão mais comuns quando importado.

Para importar esse módulo prelude, coloque esta linha perto do topo do `lib.rs`.

```rust
use cgmath::prelude::*;
```

Criaremos as instâncias em `new()`. Usaremos algumas constantes para simplificar as coisas. Exibiremos nossas instâncias em 10 linhas de 10, e elas serão espaçadas uniformemente.

```rust
const NUM_INSTANCES_PER_ROW: u32 = 10;
const INSTANCE_DISPLACEMENT: cgmath::Vector3<f32> = cgmath::Vector3::new(NUM_INSTANCES_PER_ROW as f32 * 0.5, 0.0, NUM_INSTANCES_PER_ROW as f32 * 0.5);
```

Agora, podemos criar as instâncias propriamente ditas.

```rust
impl State {
    async fn new(window: Arc<Window>) -> anyhow::Result<State> {
        // ...
        let instances = (0..NUM_INSTANCES_PER_ROW).flat_map(|z| {
            (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                let position = cgmath::Vector3 { x: x as f32, y: 0.0, z: z as f32 } - INSTANCE_DISPLACEMENT;

                let rotation = if position.is_zero() {
                    // isso é necessário para que um objeto em (0, 0, 0) não seja escalado para zero
                    // pois Quaternions podem afetar a escala se não forem criados corretamente
                    cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
                } else {
                    cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
                };

                Instance {
                    position, rotation,
                }
            })
        }).collect::<Vec<_>>();
        // ...
    }
}
```

Agora que temos nossos dados, podemos criar o `instance_buffer` propriamente dito.

```rust
let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
let instance_buffer = device.create_buffer_init(
    &wgpu::util::BufferInitDescriptor {
        label: Some("Instance Buffer"),
        contents: bytemuck::cast_slice(&instance_data),
        usage: wgpu::BufferUsages::VERTEX,
    }
);
```

Precisaremos criar um novo `VertexBufferLayout` para `InstanceRaw`.

```rust
impl InstanceRaw {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // Precisamos mudar o step_mode de Vertex para Instance
            // Isso significa que nossos shaders só mudarão para usar a próxima
            // instância quando o shader começar a processar uma nova instância
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // Um mat4 ocupa 4 slots de vértices pois é tecnicamente 4 vec4s. Precisamos definir um slot
                // para cada vec4. Teremos que remontar o mat4 no shader.
                wgpu::VertexAttribute {
                    offset: 0,
                    // Enquanto nosso vertex shader usa apenas as locations 0 e 1 agora, em tutoriais futuros
                    // usaremos 2, 3 e 4 para Vertex. Começaremos no slot 5 para não conflitar com eles mais tarde
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}
```

Precisamos adicionar esse descritor ao pipeline de renderização para que possamos usá-lo ao renderizar.

```rust
let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    // ...
    vertex: wgpu::VertexState {
        // ...
        // ATUALIZADO!
        buffers: &[Some(Vertex::desc()), Some(InstanceRaw::desc())],
    },
    // ...
});
```

Não se esqueça de retornar nossas novas variáveis!

```rust
Self {
    // ...
    // NOVO!
    instances,
    instance_buffer,
}
```

A última alteração que precisamos fazer é no método `render()`. Precisamos vincular nosso `instance_buffer` e alterar o intervalo que estamos usando em `draw_indexed()` para incluir o número de instâncias.

```rust
render_pass.set_pipeline(&self.render_pipeline);
render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
// NOVO!
render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

// ATUALIZADO!
render_pass.draw_indexed(0..self.num_indices, 0, 0..self.instances.len() as _);
```

<Note class="warning">

Certifique-se de que, se você adicionar novas instâncias ao `Vec`, você recrie tanto o `instance_buffer` quanto o `camera_bind_group`. Caso contrário, suas novas instâncias não aparecerão corretamente.

</Note>

Precisamos referenciar as partes da nossa nova matriz em `shader.wgsl` para que possamos usá-la em nossas instâncias. Adicione o seguinte ao topo do `shader.wgsl`.

```wgsl
struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
};
```

Precisamos remontar a matriz antes de podermos usá-la.

```wgsl
@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    // Continua...
}
```

Aplicaremos a `model_matrix` antes de aplicar `camera_uniform.view_proj`. Fazemos isso porque o `camera_uniform.view_proj` altera o sistema de coordenadas de `world space` (espaço do mundo) para `camera space` (espaço da câmera). Nossa `model_matrix` é uma transformação em `world space`, então não queremos estar em `camera space` ao usá-la.

```wgsl
@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    // ...
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = camera.view_proj * model_matrix * vec4<f32>(model.position, 1.0);
    return out;
}
```

Com tudo isso feito, devemos ter uma floresta de árvores!

![./forest.png](./forest.png)

## Demonstração

<WasmExample example="tutorial7_instancing"></WasmExample>

<AutoGithubLink/>

## Desafio

Modifique a posição e/ou rotação das instâncias a cada frame.
