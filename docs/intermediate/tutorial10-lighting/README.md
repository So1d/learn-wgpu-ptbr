# Trabalhando com Luzes

Embora possamos perceber que nossa cena é 3D por causa da nossa câmera, ela ainda parece muito plana. Isso acontece porque nosso modelo mantém a mesma cor, independentemente da sua orientação. Se quisermos mudar isso, precisamos adicionar iluminação à nossa cena.

No mundo real, uma fonte de luz emite fótons que rebatem até entrarem em nossos olhos. A cor que vemos é a cor original da luz menos qualquer energia que ela perdeu enquanto rebatia por aí.

No mundo da computação gráfica, modelar fótons individuais seria absurdamente custoso computacionalmente. Uma única lâmpada de 100 Watts emite cerca de 3,27 x 10^20 fótons *por segundo*. Imagine isso para o sol! Para contornar essa limitação, vamos usar matemática para "trapacear".

Vamos discutir algumas opções.

## Ray/Path Tracing

Este é um tópico *avançado*, e não o abordaremos em detalhes aqui. É o modelo mais próximo de como a luz realmente funciona, por isso achei importante mencioná-lo. Confira o [tutorial de ray tracing](../../todo/) se quiser saber mais.

## O Modelo Blinn-Phong

O Ray/Path Tracing geralmente é muito custoso computacionalmente para a maioria das aplicações em tempo real (embora isso esteja começando a mudar), por isso um método mais eficiente, embora menos preciso, baseado no [modelo de reflexão de Phong](https://pt.wikipedia.org/wiki/Sombra_Phong) é frequentemente usado. Ele divide o cálculo de iluminação em três partes: iluminação ambiente (ambient), iluminação difusa (diffuse) e iluminação especular (specular). Vamos aprender o [modelo Blinn-Phong](https://en.wikipedia.org/wiki/Blinn%E2%80%93Phong_reflection_model), que trapaceia um pouco no cálculo especular para agilizar as coisas.

Antes de entrarmos nisso, porém, precisamos adicionar uma luz à nossa cena.

```rust
// lib.rs
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LightUniform {
    position: [f32; 3],
    // Devido aos uniforms exigirem espaçamento de 16 bytes (4 floats), precisamos usar um campo de padding aqui
    _padding: u32,
    color: [f32; 3],
    // Devido aos uniforms exigirem espaçamento de 16 bytes (4 floats), precisamos usar um campo de padding aqui
    _padding2: u32,
}
```

Nosso `LightUniform` representa um ponto colorido no espaço. Vamos usar apenas luz branca pura, mas é bom permitir cores de luz diferentes.


<Note>

A regra geral para alinhamento com structs WGSL é que os alinhamentos de campos são sempre potências de 2. Por exemplo, um `vec3` pode ter apenas três campos float, o que lhe dá um tamanho de 12. O alinhamento será elevado para a próxima potência de 2, que é 16. Isso significa que você precisa ter mais cuidado com a forma como organiza sua struct em Rust.

Alguns desenvolvedores optam por usar `vec4` em vez de `vec3` para evitar problemas de alinhamento.
Você pode aprender mais sobre as regras de alinhamento na [especificação do WGSL](https://www.w3.org/TR/WGSL/#alignment-and-size).

</Note>

Vamos criar outro buffer para armazenar nossa luz.

```rust
let light_uniform = LightUniform {
    position: [2.0, 2.0, 2.0],
    _padding: 0,
    color: [1.0, 1.0, 1.0],
    _padding2: 0,
};

 // Vamos querer atualizar a posição das nossas luzes, por isso usamos COPY_DST
let light_buffer = device.create_buffer_init(
    &wgpu::util::BufferInitDescriptor {
        label: Some("Light VB"),
        contents: bytemuck::cast_slice(&[light_uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    }
);
```


Não se esqueça de adicionar o `light_uniform` e o `light_buffer` ao `State`. Depois disso, precisamos criar um bind group layout e um bind group para nossa luz.

```rust
let light_bind_group_layout =
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
        label: None,
    });

let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    layout: &light_bind_group_layout,
    entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: light_buffer.as_entire_binding(),
    }],
    label: None,
});
```

Adicione-os ao `State` e também atualize o `render_pipeline_layout`.

```rust
let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    bind_group_layouts: &[
        Some(&texture_bind_group_layout), 
        Some(&camera_bind_group_layout),
        Some(&light_bind_group_layout),
    ],
});
```

Vamos também atualizar a posição da luz no método `update()` para ver como nossos objetos se parecem a partir de diferentes ângulos.

```rust
// Atualiza a luz
let old_position: cgmath::Vector3<_> = self.light_uniform.position.into();
self.light_uniform.position =
    (cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(1.0))
        * old_position)
        .into();
self.queue.write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&[self.light_uniform]));
```

Isso fará a luz rotacionar ao redor da origem um grau a cada frame.

## Enxergando a luz

Para fins de depuração (debugging), seria legal podermos ver onde a luz está para garantir que a cena parece correta. Poderíamos adaptar nosso pipeline de renderização existente para desenhar a luz, mas isso provavelmente atrapalharia. Em vez disso, vamos extrair nosso código de criação de pipeline de renderização para uma nova função chamada `create_render_pipeline()`.


```rust
fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_layouts: &[Option<wgpu::VertexBufferLayout>],
    shader: wgpu::ShaderModuleDescriptor,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(shader);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: vertex_layouts,
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState {
                    alpha: wgpu::BlendComponent::REPLACE,
                    color: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            // Definir isso para qualquer coisa além de Fill exige Features::NON_FILL_POLYGON_MODE
            polygon_mode: wgpu::PolygonMode::Fill,
            // Exige Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Exige Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
        },
        depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview_mask: None,
    })
}
```

Também precisamos alterar `State::new()` para usar essa função.

```rust
let render_pipeline = {
    let shader = wgpu::ShaderModuleDescriptor {
        label: Some("Normal Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    };
    create_render_pipeline(
        &device,
        &render_pipeline_layout,
        config.format,
        Some(texture::Texture::DEPTH_FORMAT),
        &[Some(model::ModelVertex::desc()), Some(InstanceRaw::desc())],
        shader,
    )
};
```

Precisaremos modificar `model::DrawModel` para usar nosso `light_bind_group`.

```rust
// model.rs
pub trait DrawModel<'a> {
    fn draw_mesh(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );

    fn draw_model(
        &mut self,
        model: &'a Model,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_model_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_mesh_instanced(mesh, material, 0..1, camera_bind_group, light_bind_group);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, camera_bind_group, &[]);
        self.set_bind_group(2, light_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_model(
        &mut self,
        model: &'b Model,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_model_instanced(model, 0..1, camera_bind_group, light_bind_group);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(mesh, material, instances.clone(), camera_bind_group, light_bind_group);
        }
    }
}
```

Com isso feito, podemos criar outro pipeline de renderização para a nossa luz.

```rust
// lib.rs
let light_render_pipeline = {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Light Pipeline Layout"),
        bind_group_layouts: &[Some(&camera_bind_group_layout), Some(&light_bind_group_layout)],
        immediate_size: 0,
    });
    let shader = wgpu::ShaderModuleDescriptor {
        label: Some("Light Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("light.wgsl").into()),
    };
    create_render_pipeline(
        &device,
        &layout,
        config.format,
        Some(texture::Texture::DEPTH_FORMAT),
        &[Some(model::ModelVertex::desc())],
        shader,
    )
};
```

Escolhi criar um layout separado para o `light_render_pipeline`, pois ele não precisa de todos os recursos que o `render_pipeline` comum necessita (principalmente as texturas).

Com isso definido, precisamos escrever os shaders reais.

```wgsl
// light.wgsl
// Vertex shader

struct Camera {
    view_proj: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: Camera;

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
}
@group(1) @binding(0)
var<uniform> light: Light;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    let scale = 0.25;
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(model.position * scale + light.position, 1.0);
    out.color = light.color;
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
```

Agora, poderíamos implementar manualmente o código de desenho da luz em `render()`, mas para manter o padrão que desenvolvemos, vamos criar uma nova trait chamada `DrawLight`.

```rust
// model.rs
pub trait DrawLight<'a> {
    fn draw_light_mesh(
        &mut self,
        mesh: &'a Mesh,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_light_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );

    fn draw_light_model(
        &mut self,
        model: &'a Model,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_light_model_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawLight<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_light_mesh(
        &mut self,
        mesh: &'b Mesh,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_light_mesh_instanced(mesh, 0..1, camera_bind_group, light_bind_group);
    }

    fn draw_light_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, camera_bind_group, &[]);
        self.set_bind_group(1, light_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_light_model(
        &mut self,
        model: &'b Model,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_light_model_instanced(model, 0..1, camera_bind_group, light_bind_group);
    }
    fn draw_light_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            self.draw_light_mesh_instanced(mesh, instances.clone(), camera_bind_group, light_bind_group);
        }
    }
}
```

Por fim, queremos adicionar a renderização da Luz aos nossos render passes.

```rust
impl State {
    // ...
   fn render(&mut self) -> anyhow::Result<()> {
        // ...
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

        use crate::model::DrawLight; // NOVO!
        render_pass.set_pipeline(&self.light_render_pipeline); // NOVO!
        render_pass.draw_light_model(
            &self.obj_model,
            &self.camera_bind_group,
            &self.light_bind_group,
        ); // NOVO!

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.draw_model_instanced(
            &self.obj_model,
            0..self.instances.len() as u32,
            &self.camera_bind_group,
            &self.light_bind_group, // NOVO
        );
}
```

Com tudo isso, terminaremos com algo assim.

![./light-in-scene.png](./light-in-scene.png)

## Iluminação Ambiente (Ambient)

A luz tem uma tendência a rebater por aí antes de entrar em nossos olhos. É por isso que você consegue ver em áreas que estão na sombra. Modelar essa interação seria computacionalmente custoso, por isso vamos trapacear. Definimos um valor de iluminação ambiente para a luz que rebate de outras partes da cena para iluminar nossos objetos.

A parte ambiente é baseada na cor da luz e na cor do objeto. Já adicionamos nosso `light_bind_group`, portanto só precisamos usá-lo em nosso shader. Em `shader.wgsl`, adicione o seguinte abaixo dos uniforms de textura.

```wgsl
struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
}
@group(2) @binding(0)
var<uniform> light: Light;
```

Em seguida, precisamos atualizar o código principal do shader para calcular e usar o valor da cor ambiente.

```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    
    // Não precisamos (nem queremos) muita luz ambiente, então 0.1 está ótimo
    let ambient_strength = 0.1;
    let ambient_color = light.color * ambient_strength;

    let result = ambient_color * object_color.xyz;

    return vec4<f32>(result, object_color.a);
}
```

Com isso, devemos obter algo assim.

![./ambient_lighting.png](./ambient_lighting.png)

## Iluminação Difusa (Diffuse)

Lembra dos vetores normais que estavam incluídos em nosso modelo? Finalmente vamos usá-los. As normais representam a direção para a qual uma superfície está voltada. Comparando a normal de um fragmento com um vetor apontando para a fonte de luz, obtemos um valor de quão claro/escuro esse fragmento deve ser. Comparamos os vetores usando o produto escalar (dot product) para obter o cosseno do ângulo entre eles.

![./normal_diagram.png](./normal_diagram.png)

Se o produto escalar da normal e do vetor de luz for 1.0, isso significa que o fragmento atual está diretamente alinhado com a fonte de luz e receberá a intensidade total da luz. Um valor de 0.0 ou inferior significa que a superfície é perpendicular ou está voltada de costas para a luz e, portanto, ficará escura.

Vamos precisar trazer o vetor normal para o nosso `shader.wgsl`.

```wgsl
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>, // NOVO!
};
```

Também vamos querer passar esse valor, bem como a posição do vértice, para o fragment shader.

```wgsl
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
};
```

Por enquanto, vamos apenas passar a normal diretamente como ela é. Isso está incorreto, mas corrigiremos isso mais tarde.

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
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.world_normal = model.normal;
    var world_position: vec4<f32> = model_matrix * vec4<f32>(model.position, 1.0);
    out.world_position = world_position.xyz;
    out.clip_position = camera.view_proj * world_position;
    return out;
}
```

Com isso, podemos fazer o cálculo real. Adicione o seguinte abaixo do cálculo de `ambient_color`, mas acima de `result`.

```wgsl
let light_dir = normalize(light.position - in.world_position);

let diffuse_strength = max(dot(in.world_normal, light_dir), 0.0);
let diffuse_color = light.color * diffuse_strength;
```

Agora podemos incluir `diffuse_color` no `result`.

```wgsl
let result = (ambient_color + diffuse_color) * object_color.xyz;
```

Com isso, obtemos algo assim.

![./ambient_diffuse_wrong.png](./ambient_diffuse_wrong.png)

## A Matriz de Normais (Normal Matrix)

Lembra quando eu disse que passar a normal do vértice diretamente para o fragment shader estava incorreto? Vamos explorar isso removendo todos os cubos da cena, exceto um que será rotacionado em 180 graus no eixo y.

```rust
const NUM_INSTANCES_PER_ROW: u32 = 1;

// No loop onde criamos as instâncias
let rotation = cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(180.0));
```

Também removeremos a `ambient_color` do nosso `result` de iluminação.

```wgsl
let result = (diffuse_color) * object_color.xyz;
```

Isso deve nos dar algo parecido com isto.

![./diffuse_wrong.png](./diffuse_wrong.png)

Isso está claramente errado, já que a luz está iluminando o lado errado do cubo. Isso ocorre porque não estamos rotacionando nossas normais com nosso objeto, então não importa em qual direção o objeto esteja voltado, as normais sempre estarão voltadas para a mesma direção.

![./normal_not_rotated.png](./normal_not_rotated.png)

Precisamos usar a matriz de modelo (model matrix) para transformar as normais na direção correta. Queremos apenas os dados de rotação, no entanto. Uma normal representa uma direção e deve ser um vetor unitário durante todo o cálculo. Podemos colocar nossas normais na direção certa usando o que é chamado de matriz de normais (normal matrix).

Poderíamos calcular a matriz de normais no vertex shader, mas isso envolveria inverter a `model_matrix`, e o WGSL não tem uma função de inversão integrada. Teríamos que codificar a nossa própria. Além disso, calcular a inversa de uma matriz é realmente custoso, especialmente fazer esse cálculo para cada vértice.

Em vez disso, vamos adicionar um campo de matriz `normal` em `InstanceRaw`. Em vez de inverter a matriz de modelo, vamos apenas usar a rotação da instância para criar uma `Matrix3`.

<Note>

Estamos usando `Matrix3` em vez de `Matrix4` pois precisamos realmente apenas do componente de rotação da matriz.

</Note>

```rust
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[allow(dead_code)]
struct InstanceRaw {
    model: [[f32; 4]; 4],
    normal: [[f32; 3]; 3],
}

impl model::Vertex for InstanceRaw {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // Precisamos mudar o step_mode de Vertex para Instance
            // Isso significa que nossos shaders só avançarão para a próxima
            // instância quando o shader começar a processar uma nova instância
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    // Embora nosso vertex shader use apenas as locations 0 e 1 por enquanto, nos tutoriais futuros
                    // usaremos 2, 3 e 4 para o Vertex. Começaremos no slot 5 para não ter conflito mais tarde
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // Um mat4 ocupa 4 slots de vértices pois é tecnicamente 4 vec4s. Precisamos definir um slot
                // para cada vec4. Mas não precisamos fazer isso no código.
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
                // NOVO!
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 19]>() as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 22]>() as wgpu::BufferAddress,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}
```

Precisamos modificar `Instance` para criar a matriz de normais.

```rust
struct Instance {
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
}

impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        let model =
            cgmath::Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation);
        InstanceRaw {
            model: model.into(),
            // NOVO!
            normal: cgmath::Matrix3::from(self.rotation).into(),
        }
    }
}
```

Agora, precisamos reconstruir a matriz de normais no vertex shader.

```wgsl
struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    // NOVO!
    @location(9) normal_matrix_0: vec3<f32>,
    @location(10) normal_matrix_1: vec3<f32>,
    @location(11) normal_matrix_2: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
};

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
    // NOVO!
    let normal_matrix = mat3x3<f32>(
        instance.normal_matrix_0,
        instance.normal_matrix_1,
        instance.normal_matrix_2,
    );
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.world_normal = normal_matrix * model.normal; // ATUALIZADO!
    var world_position: vec4<f32> = model_matrix * vec4<f32>(model.position, 1.0);
    out.world_position = world_position.xyz;
    out.clip_position = camera.view_proj * world_position;
    return out;
}
```

<Note>

No momento estou fazendo as coisas no [espaço do mundo (world space)](https://gamedev.stackexchange.com/questions/65783/what-are-world-space-and-eye-space-in-game-development). Fazer as coisas no espaço da câmera/visão (view-space ou eye-space) é o mais padrão, já que os objetos podem ter problemas de iluminação quando estão mais distantes da origem. Se quiséssemos usar o view-space, teríamos incluído também a rotação da matriz de visão (view matrix). Também teríamos que transformar a posição da nossa luz usando algo como `view_matrix * model_matrix * light_position` para evitar que o cálculo seja prejudicado quando a câmera se move.

Existem vantagens em usar o view space. A principal é que, quando você tem mundos gigantescos fazendo iluminação e outros cálculos no espaço do modelo, isso pode causar problemas à medida que a precisão de ponto flutuante se degrada quando os números ficam muito grandes. O view space mantém a câmera na origem, o que significa que todos os cálculos usarão números menores. A matemática de iluminação em si permanece a mesma, mas exige um pouco mais de configuração.

</Note>

Com essa alteração, nossa iluminação agora parece correta.

![./diffuse_right.png](./diffuse_right.png)

Trazendo de volta nossos outros objetos e adicionando a iluminação ambiente temos isto.

![./ambient_diffuse_lighting.png](./ambient_diffuse_lighting.png);

<Note>

Se você puder garantir que sua matriz de modelo sempre aplicará um dimensionamento (scaling) uniforme aos seus objetos, você pode usar apenas a matriz de modelo. O usuário do Github @julhe compartilhou comigo este código que resolve o problema:

```wgsl
out.world_normal = (model_matrix * vec4<f32>(model.normal, 0.0)).xyz;
```

Isso funciona explorando o fato de que ao multiplicar uma matriz 4x4 por um vetor com 0 no componente w, apenas a rotação e a escala serão aplicadas ao vetor. Você precisará normalizar esse vetor, no entanto, pois as normais precisam ser de comprimento unitário para que os cálculos funcionem.

O fator de escala *precisa* ser uniforme para que isso funcione. Caso contrário, a normal resultante ficará distorcida, como você pode ver na imagem a seguir.

![./normal-scale-issue.png](./normal-scale-issue.png)

</Note>

## Iluminação Especular (Specular)

A iluminação especular descreve os destaques (highlights/brilhos) que aparecem nos objetos quando vistos de determinados ângulos. Se você já olhou para um carro, são aquelas partes super brilhantes. Basicamente, parte da luz pode refletir na superfície como um espelho. A localização do destaque muda dependendo do ângulo pelo qual você o observa.

![./specular_diagram.png](./specular_diagram.png)

Como isso é relativo ao ângulo de visão, precisaremos passar a posição da câmera tanto para o fragment shader quanto para o vertex shader.

```wgsl
struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}
@group(1) @binding(0)
var<uniform> camera: Camera;
```

<Note>

Não se esqueça de atualizar a struct `Camera` em `light.wgsl` também, pois se ela não corresponder à struct `CameraUniform` em Rust, a luz será renderizada incorretamente.

</Note>

Precisaremos atualizar a struct `CameraUniform` também.

```rust
// lib.rs
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view_position: [0.0; 4],
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        // Estamos usando Vector4 por causa do requisito de espaçamento de 16 bytes do uniform
        self.view_position = camera.eye.to_homogeneous().into();
        self.view_proj = (OPENGL_TO_WGPU_MATRIX * camera.build_view_projection_matrix()).into();
    }
}
```

Como queremos usar nossos uniforms no fragment shader agora, precisamos alterar sua visibilidade.

```rust
// lib.rs
let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    entries: &[
        wgpu::BindGroupLayoutBinding {
            // ...
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT, // Atualizado!
            // ...
        },
        // ...
    ],
    label: None,
});
```

Vamos obter a direção da posição do fragmento até a câmera e usar isso com a normal para calcular a `reflect_dir`.

```wgsl
// shader.wgsl
// No fragment shader...
let view_dir = normalize(camera.view_pos.xyz - in.world_position);
let reflect_dir = reflect(-light_dir, in.world_normal);
```

Em seguida, usamos o produto escalar para calcular `specular_strength` e usamos isso para calcular `specular_color`.

```wgsl
let specular_strength = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);
let specular_color = specular_strength * light.color;
```

Por fim, adicionamos isso ao resultado.

```wgsl
let result = (ambient_color + diffuse_color + specular_color) * object_color.xyz;
```

Com isso, você deve ter algo parecido com isto.

![./ambient_diffuse_specular_lighting.png](./ambient_diffuse_specular_lighting.png)

Se olharmos apenas para `specular_color` isoladamente, obtemos isto.

![./specular_lighting.png](./specular_lighting.png)

## A direção intermediária (Half Direction)

Até este ponto, implementamos apenas a parte Phong do Blinn-Phong. O modelo de reflexão de Phong funciona bem, mas pode falhar em [certas circunstâncias](https://learnopengl.com/Advanced-Lighting/Advanced-Lighting). A parte Blinn de Blinn-Phong vem da percepção de que se você somar `view_dir` e `light_dir`, normalizar o resultado e usar o produto escalar disso com a `normal`, você obterá aproximadamente os mesmos resultados sem os problemas que o uso de `reflect_dir` causava.

```wgsl
let view_dir = normalize(camera.view_pos.xyz - in.world_position);
let half_dir = normalize(view_dir + light_dir);

let specular_strength = pow(max(dot(in.world_normal, half_dir), 0.0), 32.0);
```

É difícil notar a diferença, mas aqui estão os resultados.

![./half_dir.png](./half_dir.png)

## Demonstração

<WasmExample example="tutorial10_lighting"></WasmExample>

<AutoGithubLink/>
