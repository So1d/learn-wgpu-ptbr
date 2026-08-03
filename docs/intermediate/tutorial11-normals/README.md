# Mapeamento de Normais (Normal Mapping)

Apenas com a iluminação, nossa cena já está com um aspecto muito bom. No entanto, nossos modelos ainda parecem excessivamente lisos. Isso é compreensível porque estamos usando um modelo 3D muito simples. Se estivéssemos usando uma textura que deveria ser lisa, isso não seria um problema, mas nossa textura de tijolos deveria ser mais rústica e áspera. Poderíamos resolver isso adicionando mais geometria, mas isso desaceleraria nossa cena e seria difícil saber onde adicionar novos polígonos. É aqui que entra o mapeamento de normais (normal mapping).

Lembra de quando experimentamos armazenar dados de instâncias em uma textura no [tutorial de instanciamento](/beginner/tutorial7-instancing/#uma-forma-diferente-texturas)? Um normal map faz exatamente isso com dados de normais! Usaremos as normais contidas no normal map em nosso cálculo de iluminação além da normal do vértice.

A textura de tijolos que encontrei veio com um normal map. Vamos dar uma olhada nele!

![./cube-normal.png](./cube-normal.png)

Os componentes r, g e b da textura correspondem aos componentes x, y e z das normais. Todos os valores z devem ser positivos. É por isso que o normal map tem uma tonalidade azulada.

Precisaremos modificar nossa struct `Material` em `model.rs` para incluir uma `normal_texture`.

```rust
pub struct Material {
    pub name: String,
    pub diffuse_texture: texture::Texture,
    pub normal_texture: texture::Texture, // ATUALIZADO!
    pub bind_group: wgpu::BindGroup,
}
```

Teremos que atualizar o `texture_bind_group_layout` para incluir o normal map também.

```rust
let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    entries: &[
        // ...
        // normal map
        wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                multisampled: false,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 3,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        },
    ],
    label: Some("texture_bind_group_layout"),
});
```

Precisaremos carregar o normal map. Faremos isso no loop onde criamos os materiais na função `load_model()` em `resources.rs`.

```rust
// resources.rs
let mut materials = Vec::new();
for m in obj_materials? {
    let diffuse_texture = load_texture(&m.diffuse_texture, device, queue).await?;
    // NOVO!
    let normal_texture = load_texture(&m.normal_texture, device, queue).await?;

    materials.push(model::Material::new(
        device,
        &m.name,
        diffuse_texture,
        normal_texture, // NOVO!
        layout,
    ));
}
```

Você notará que estou usando uma função `Material::new()` que não tínhamos anteriormente. Aqui está o código para ela:

```rust
impl Material {
    pub fn new(
        device: &wgpu::Device,
        name: &str,
        diffuse_texture: texture::Texture,
        normal_texture: texture::Texture, // NOVO!
        layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
                // NOVO!
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&normal_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&normal_texture.sampler),
                },
            ],
            label: Some(name),
        });

        Self {
            name: String::from(name),
            diffuse_texture,
            normal_texture, // NOVO!
            bind_group,
        }
    }
}
```

Agora, podemos usar a textura no fragment shader.

```wgsl
// Fragment shader

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0)@binding(1)
var s_diffuse: sampler;
@group(0)@binding(2)
var t_normal: texture_2d<f32>;
@group(0) @binding(3)
var s_normal: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let object_normal: vec4<f32> = textureSample(t_normal, s_normal, in.tex_coords);
    
    // Não precisamos (nem queremos) muita luz ambiente, então 0.1 está ótimo
    let ambient_strength = 0.1;
    let ambient_color = light.color * ambient_strength;

    // Cria os vetores de iluminação
    let tangent_normal = object_normal.xyz * 2.0 - 1.0;
    let light_dir = normalize(light.position - in.world_position);
    let view_dir = normalize(camera.view_pos.xyz - in.world_position);
    let half_dir = normalize(view_dir + light_dir);

    let diffuse_strength = max(dot(tangent_normal, light_dir), 0.0);
    let diffuse_color = light.color * diffuse_strength;

    let specular_strength = pow(max(dot(tangent_normal, half_dir), 0.0), 32.0);
    let specular_color = specular_strength * light.color;

    let result = (ambient_color + diffuse_color + specular_color) * object_color.xyz;

    return vec4<f32>(result, object_color.a);
}
```

Se executarmos o código agora, você notará que as coisas não parecem muito certas. Vamos comparar nossos resultados com o último tutorial.

![](./normal_mapping_wrong.png)
![](./ambient_diffuse_specular_lighting.png)

Partes da cena estão escuras quando deveriam estar iluminadas, e vice-versa.

## Espaço Tangente para Espaço do Mundo (Tangent Space to World Space)

Mencionei brevemente no [tutorial de iluminação](/intermediate/tutorial10-lighting/#the-normal-matrix) que estávamos fazendo nosso cálculo de iluminação no "espaço do mundo" (world space). Isso significava que toda a cena estava orientada em relação ao sistema de coordenadas do *mundo*. Quando extraímos os dados de normais da nossa textura de normais, todas as normais estão no que é conhecido como apontando aproximadamente na direção z positiva. Isso significa que nosso cálculo de iluminação pensa que todas as superfícies dos nossos modelos estão voltadas para a mesma direção. Isso é chamado de `espaço tangente` (tangent space).

Se nos lembrarmos do [tutorial de iluminação](/intermediate/tutorial10-lighting/#), usamos a normal do vértice para indicar a direção da superfície. Acontece que podemos usar isso para transformar nossas normais do `espaço tangente` para o `espaço do mundo`. Para fazer isso, precisamos recorrer à álgebra linear.

Podemos criar uma matriz que representa um sistema de coordenadas usando três vetores que são perpendiculares (ou ortonormais) entre si. Basicamente, definimos os eixos x, y e z do nosso sistema de coordenadas.

```wgsl
let coordinate_system = mat3x3<f32>(
    vec3(1, 0, 0), // eixo x (direita / right)
    vec3(0, 1, 0), // eixo y (cima / up)
    vec3(0, 0, 1)  // eixo z (frente / forward)
);
```

Vamos criar uma matriz que representará o espaço de coordenadas relativo às normais dos nossos vértices. Em seguida, usaremos essa matriz para transformar os dados do nosso normal map para ficarem no espaço do mundo.

## A tangente e a bitangente

Temos um dos três vetores de que precisamos: a normal. E quanto aos outros? Estes são os vetores tangente e bitangente. Uma tangente representa qualquer vetor paralelo a uma superfície (ou seja, que não a intersecta). A tangente é sempre perpendicular ao vetor normal. A bitangente é um vetor tangente perpendicular ao outro vetor tangente. Juntos, a tangente, a bitangente e a normal representam os eixos x, y e z, respectivamente.

Alguns formatos de modelo 3D incluem a tangente e a bitangente (às vezes chamada de binormal) nos dados de vértices, mas o formato OBJ não inclui. Teremos que calculá-las manualmente. Felizmente, podemos derivar nossa tangente e bitangente a partir dos nossos dados de vértices existentes. Observe o diagrama a seguir.

![](./tangent_space.png)

Basicamente, podemos usar as arestas dos nossos triângulos e nossa normal para calcular a tangente e a bitangente. Mas primeiro, precisamos atualizar nossa struct `ModelVertex` em `model.rs`.

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
    // NOVO!
    pub tangent: [f32; 3],
    pub bitangent: [f32; 3],
}
```

Precisaremos atualizar nosso `VertexBufferLayout` também.

```rust
impl Vertex for ModelVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // ...

                // Tangente e bitangente
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 11]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}
```

Agora, podemos calcular os novos vetores tangente e bitangente. Atualize a geração de malhas em `load_model()` em `resources.rs` para usar o seguinte código:

```rust
let meshes = models
    .into_iter()
    .map(|m| {
        let mut vertices = (0..m.mesh.positions.len() / 3)
            .map(|i| model::ModelVertex {
                position: [
                    m.mesh.positions[i * 3],
                    m.mesh.positions[i * 3 + 1],
                    m.mesh.positions[i * 3 + 2],
                ],
                tex_coords: [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]],
                normal: [
                    m.mesh.normals[i * 3],
                    m.mesh.normals[i * 3 + 1],
                    m.mesh.normals[i * 3 + 2],
                ],
                // Calcularemos estes mais tarde
                tangent: [0.0; 3],
                bitangent: [0.0; 3],
            })
            .collect::<Vec<_>>();

        let indices = &m.mesh.indices;
        let mut triangles_included = vec![0; vertices.len()];

        // Calcula tangentes e bitangentes. Vamos usar os
        // triângulos, portanto precisamos percorrer os
        // índices em blocos de 3
        for c in indices.chunks(3) {
            let v0 = vertices[c[0] as usize];
            let v1 = vertices[c[1] as usize];
            let v2 = vertices[c[2] as usize];

            let pos0: cgmath::Vector3<_> = v0.position.into();
            let pos1: cgmath::Vector3<_> = v1.position.into();
            let pos2: cgmath::Vector3<_> = v2.position.into();

            let uv0: cgmath::Vector2<_> = v0.tex_coords.into();
            let uv1: cgmath::Vector2<_> = v1.tex_coords.into();
            let uv2: cgmath::Vector2<_> = v2.tex_coords.into();

            // Calcula as arestas do triângulo
            let delta_pos1 = pos1 - pos0;
            let delta_pos2 = pos2 - pos0;

            // Isso nos dará uma direção para calcular a
            // tangente e a bitangente
            let delta_uv1 = uv1 - uv0;
            let delta_uv2 = uv2 - uv0;

            // Resolver o seguinte sistema de equações nos
            // dará a tangente e a bitangente.
            //     delta_pos1 = delta_uv1.x * T + delta_u.y * B
            //     delta_pos2 = delta_uv2.x * T + delta_uv2.y * B
            // Felizmente, o local onde encontrei essa equação forneceu
            // a solução!
            let r = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);
            let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
            // Invertemos a bitangente para permitir normal maps
            // para a mão direita (right-handed) no sistema de coordenadas de textura do wgpu
            let bitangent = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * -r;

            // Usaremos a mesma tangente/bitangente para cada vértice do triângulo
            vertices[c[0] as usize].tangent =
                (tangent + cgmath::Vector3::from(vertices[c[0] as usize].tangent)).into();
            vertices[c[1] as usize].tangent =
                (tangent + cgmath::Vector3::from(vertices[c[1] as usize].tangent)).into();
            vertices[c[2] as usize].tangent =
                (tangent + cgmath::Vector3::from(vertices[c[2] as usize].tangent)).into();
            vertices[c[0] as usize].bitangent =
                (bitangent + cgmath::Vector3::from(vertices[c[0] as usize].bitangent)).into();
            vertices[c[1] as usize].bitangent =
                (bitangent + cgmath::Vector3::from(vertices[c[1] as usize].bitangent)).into();
            vertices[c[2] as usize].bitangent =
                (bitangent + cgmath::Vector3::from(vertices[c[2] as usize].bitangent)).into();

            // Usado para tirar a média das tangentes/bitangentes
            triangles_included[c[0] as usize] += 1;
            triangles_included[c[1] as usize] += 1;
            triangles_included[c[2] as usize] += 1;
        }

        // Tira a média das tangentes/bitangentes
        for (i, n) in triangles_included.into_iter().enumerate() {
            let denom = 1.0 / n as f32;
            let mut v = &mut vertices[i];
            v.tangent = (cgmath::Vector3::from(v.tangent) * denom).into();
            v.bitangent = (cgmath::Vector3::from(v.bitangent) * denom).into();
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Vertex Buffer", file_name)),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Index Buffer", file_name)),
            contents: bytemuck::cast_slice(&m.mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        model::Mesh {
            name: file_name.to_string(),
            vertex_buffer,
            index_buffer,
            num_elements: m.mesh.indices.len() as u32,
            material: m.mesh.material_id.unwrap_or(0),
        }
    })
    .collect::<Vec<_>>();
```

## Espaço do Mundo para Espaço Tangente (World Space to Tangent Space)

Como o normal map, por padrão, está no espaço tangente, precisamos transformar todas as outras variáveis usadas nesse cálculo para o espaço tangente também. Precisaremos construir a matriz tangente no vertex shader. Primeiro, precisamos que nossa `VertexInput` inclua a tangente e as bitangentes que calculamos anteriormente.

```wgsl
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
};
```

Em seguida, construiremos a `tangent_matrix` e depois transformaremos a luz do vértice e a posição de visão (view position) para o espaço tangente.

```wgsl
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    // ATUALIZADO!
    @location(1) tangent_position: vec3<f32>,
    @location(2) tangent_light_position: vec3<f32>,
    @location(3) tangent_view_position: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    // ...
    let normal_matrix = mat3x3<f32>(
        instance.normal_matrix_0,
        instance.normal_matrix_1,
        instance.normal_matrix_2,
    );

    // Constrói a matriz tangente
    let world_normal = normalize(normal_matrix * model.normal);
    let world_tangent = normalize(normal_matrix * model.tangent);
    let world_bitangent = normalize(normal_matrix * model.bitangent);
    let tangent_matrix = transpose(mat3x3<f32>(
        world_tangent,
        world_bitangent,
        world_normal,
    ));

    let world_position = model_matrix * vec4<f32>(model.position, 1.0);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * world_position;
    out.tex_coords = model.tex_coords;
    out.tangent_position = tangent_matrix * world_position.xyz;
    out.tangent_view_position = tangent_matrix * camera.view_pos.xyz;
    out.tangent_light_position = tangent_matrix * light.position;
    return out;
}
```

Finalmente, atualizaremos o fragment shader para usar esses valores de iluminação transformados.

```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Amostra as texturas..

    // Cria os vetores de iluminação
    let tangent_normal = object_normal.xyz * 2.0 - 1.0;
    let light_dir = normalize(in.tangent_light_position - in.tangent_position);
    let view_dir = normalize(in.tangent_view_position - in.tangent_position);

    // Realiza cálculos de iluminação...
}
```

Obtemos o seguinte com esse cálculo.

![](./normal_mapping_correct.png)

## Srgb e Texturas de Normais

Temos usado `Rgba8UnormSrgb` para todas as nossas texturas. Srgb é um espaço de cores não linear. É ideal para monitores porque a percepção de cores humana também não é linear, e o Srgb foi projetado para corresponder às peculiaridades da percepção humana de cores.

Mas o Srgb é um espaço de cores inadequado para dados que devem ser operados matematicamente. Tais dados devem estar em um espaço de cores linear (sem correção de gama). Quando uma GPU amostra uma textura com Srgb no nome, ela converte os dados do Srgb não linear com correção de gama para um espaço de cores linear sem correção de gama primeiro, para que você possa fazer matemática com eles (e faz a conversão oposta se você escrever de volta para uma textura Srgb).

Normal maps já são armazenados em um formato linear. Portanto, devemos especificar o espaço linear para a textura, para que ela não faça uma conversão inadequada quando lermos dela.

Precisamos especificar `Rgba8Unorm` quando criamos a textura. Vamos adicionar o parâmetro `is_normal_map` ao método da nossa struct Texture.

```rust
pub fn from_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    img: &image::DynamicImage,
    label: Option<&str>,
    is_normal_map: bool, // NOVO!
) -> Result<Self> {
    // ...
    // NOVO!
    let format = if is_normal_map {
        wgpu::TextureFormat::Rgba8Unorm
    } else {
        wgpu::TextureFormat::Rgba8UnormSrgb
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label,
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        // ATUALIZADO!
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    // ...
    
    Ok(Self {
        texture,
        view,
        sampler,
    })
}
```

Precisaremos propagar essa alteração para os outros métodos que usam essa função.

```rust
pub fn from_bytes(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bytes: &[u8],
    label: &str,
    is_normal_map: bool, // NOVO!
) -> Result<Self> {
    let img = image::load_from_memory(bytes)?;
    Self::from_image(device, queue, &img, Some(label), is_normal_map) // ATUALIZADO!
}
```

Precisamos atualizar o `resources.rs` também.

```rust
pub async fn load_texture(
    file_name: &str,
    is_normal_map: bool,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> anyhow::Result<texture::Texture> {
    let data = load_binary(file_name).await?;
    texture::Texture::from_bytes(device, queue, &data, file_name, is_normal_map)
}

pub async fn load_model(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> anyhow::Result<model::Model> {
    // ...

    let mut materials = Vec::new();
    for m in obj_materials? {
        let diffuse_texture = load_texture(&m.diffuse_texture, false, device, queue).await?; // ATUALIZADO!
        let normal_texture = load_texture(&m.normal_texture, true, device, queue).await?; // ATUALIZADO!

        materials.push(model::Material::new(
            device,
            &m.name,
            diffuse_texture,
            normal_texture,
            layout,
        ));
    }
}

```

Isso nos dá o seguinte resultado.

![](./no_srgb.png)

## Recursos adicionais

Eu queria experimentar outros materiais, então adicionei um `draw_model_instanced_with_material()` à trait `DrawModel`.

```rust
pub trait DrawModel<'a> {
    // ...
    fn draw_model_instanced_with_material(
        &mut self,
        model: &'a Model,
        material: &'a Material,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    // ...
    fn draw_model_instanced_with_material(
        &mut self,
        model: &'b Model,
        material: &'b Material,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            self.draw_mesh_instanced(mesh, material, instances.clone(), camera_bind_group, light_bind_group);
        }
    }
}
```

Encontrei uma textura de paralelepípedo (cobblestone) com um normal map correspondente e criei um `debug_material` para ela.

```rust
// lib.rs
impl State {
    async fn new(window: &Window) -> Result<Self> {
        // ...
        let debug_material = {
            let diffuse_bytes = include_bytes!("../res/cobble-diffuse.png");
            let normal_bytes = include_bytes!("../res/cobble-normal.png");

            let diffuse_texture = texture::Texture::from_bytes(&device, &queue, diffuse_bytes, "res/alt-diffuse.png", false).unwrap();
            let normal_texture = texture::Texture::from_bytes(&device, &queue, normal_bytes, "res/alt-normal.png", true).unwrap();
            
            model::Material::new(&device, "alt-material", diffuse_texture, normal_texture, &texture_bind_group_layout)
        };
        Self {
            // ...
            #[allow(dead_code)]
            debug_material,
        }
    }
}
```

Então, para renderizar com o `debug_material`, usei o `draw_model_instanced_with_material()` que criei.

```rust
render_pass.set_pipeline(&self.render_pipeline);
render_pass.draw_model_instanced_with_material(
    &self.obj_model,
    &self.debug_material,
    0..self.instances.len() as u32,
    &self.camera_bind_group,
    &self.light_bind_group,
);
```

Isso nos dá algo assim.

![](./debug_material.png)

Você pode encontrar as texturas que uso no Repositório do GitHub.

## Demonstração

<WasmExample example="tutorial11_normals"></WasmExample>

<AutoGithubLink/>
