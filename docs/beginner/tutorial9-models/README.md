# Carregamento de Modelos

Até este ponto, estivemos criando nossos modelos manualmente. Embora essa seja uma forma aceitável de fazer isso, é muito lento se quisermos incluir modelos complexos com muitos polígonos. Por causa disso, vamos modificar nosso código para aproveitar o formato de modelo `.obj` para que possamos criar um modelo em um software como o Blender e exibi-lo em nosso código.

Nosso arquivo `lib.rs` está ficando bastante desordenado. Vamos criar um arquivo `model.rs` no qual podemos colocar nosso código de carregamento de modelos.

```rust
// model.rs
pub trait Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}

impl Vertex for ModelVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        todo!();
    }
}
```

Você notará algumas coisas aqui. Em `lib.rs`, tínhamos `Vertex` como uma struct, mas aqui estamos usando uma trait. Poderíamos ter múltiplos tipos de vértices (modelo, interface de usuário/UI, dados de instância, etc.). Fazer do `Vertex` uma trait nos permitirá abstrair o código de criação do `VertexBufferLayout` para tornar a criação de `RenderPipeline`s mais simples.

Outra coisa a mencionar é o campo `normal` na `ModelVertex`. Não usaremos isso até falarmos sobre iluminação, mas vamos adicioná-lo à struct por enquanto.

Vamos definir nosso `VertexBufferLayout`.

```rust
impl Vertex for ModelVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}
```

Isso é basicamente o mesmo que o `VertexBufferLayout` original, mas adicionamos um `VertexAttribute` para a `normal`. Remova a struct `Vertex` em `lib.rs`, pois não precisaremos mais dela, e use nossa nova trait `Vertex` de `model` para o `RenderPipeline`.

Também removeremos nossos `vertex_buffer`, `index_buffer` e `num_indices` feitos à mão.

```rust
let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    // ...
    vertex: wgpu::VertexState {
        // ...
        buffers: &[Some(model::ModelVertex::desc()), Some(InstanceRaw::desc())],
    },
    // ...
});
```

Como o método `desc` é implementado na trait `Vertex`, a trait precisa ser importada antes que o método fique acessível. Coloque a importação perto do topo do arquivo com as outras.

```rust
use model::Vertex;
```

Com tudo isso no lugar, precisamos de um modelo para renderizar. Se você já tiver um, ótimo, mas forneci um [arquivo zip](https://github.com/sotrh/learn-wgpu/blob/master/code/beginner/tutorial9-models/res/cube.zip) com o modelo e todas as suas texturas. Vamos colocar este modelo em uma nova pasta `res` ao lado da pasta `src` existente.

## Acessando arquivos na pasta res

Quando o Cargo compila e executa nosso programa, ele define o que é conhecido como diretório de trabalho atual (current working directory). Esse diretório geralmente contém o `Cargo.toml` raiz do seu projeto. O caminho para a nossa pasta res pode variar dependendo da estrutura do projeto. Na pasta `res`, o código de exemplo para este tutorial está em `code/beginner/tutorial9-models/res/`. Ao carregar nosso modelo, poderíamos usar esse caminho e apenas anexar `cube.obj`. Isso funciona, mas se mudarmos a estrutura do nosso projeto, nosso código quebrará.

Vamos corrigir isso modificando nosso script de compilação (build script) para copiar nossa pasta `res` para onde o Cargo cria nosso executável, e faremos referência a ela a partir de lá. Crie um arquivo chamado `build.rs` e adicione o seguinte:

```rust
use anyhow::*;
use fs_extra::copy_items;
use fs_extra::dir::CopyOptions;
use std::env;

fn main() -> Result<()> {
    // Isso instrui o Cargo a reexecutar este script se algo em /res/ mudar.
    println!("cargo:rerun-if-changed=res/*");

    let out_dir = env::var("OUT_DIR")?;
    let mut copy_options = CopyOptions::new();
    copy_options.overwrite = true;
    let mut paths_to_copy = Vec::new();
    paths_to_copy.push("res/");
    copy_items(&paths_to_copy, out_dir, &copy_options)?;

    Ok(())
}
```

<Note>

Certifique-se de colocar `build.rs` na mesma pasta do `Cargo.toml`. Se não o fizer, o Cargo não o executará quando sua crate for compilada.

</Note>

<Note>

O `OUT_DIR` é uma variável de ambiente que o Cargo usa para especificar onde nossa aplicação será compilada.

</Note>

Você precisará modificar seu `Cargo.toml` para que isso funcione corretamente. Adicione o seguinte abaixo do seu bloco `[dependencies]`.

```toml
[build-dependencies]
anyhow = "1.0"
fs_extra = "1.2"
glob = "0.3"
```

## Acessando arquivos a partir do WASM

Por design, você não pode acessar arquivos no sistema de arquivos de um usuário no WebAssembly. Em vez disso, disponibilizaremos esses arquivos usando um servidor web e depois os carregaremos em nosso código usando uma requisição HTTP. Para simplificar isso, vamos criar um arquivo chamado `resources.rs` para cuidar disso para nós. Criaremos duas funções que carregam arquivos de texto e binários, respectivamente.

```rust
use std::io::{BufReader, Cursor};

use wgpu::util::DeviceExt;

use crate::{model, texture};

#[cfg(target_arch = "wasm32")]
fn format_url(file_name: &str) -> reqwest::Url {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let mut origin = location.origin().unwrap();
    if !origin.ends_with("learn-wgpu") {
        origin = format!("{}/learn-wgpu", origin);
    }
    let base = reqwest::Url::parse(&format!("{}/", origin,)).unwrap();
    base.join(file_name).unwrap()
}

pub async fn load_string(file_name: &str) -> anyhow::Result<String> {
    #[cfg(target_arch = "wasm32")]
    let txt = {
        let url = format_url(file_name);
        reqwest::get(url).await?.text().await?
    };
    #[cfg(not(target_arch = "wasm32"))]
    let txt = {
        let path = std::path::Path::new(env!("OUT_DIR"))
            .join("res")
            .join(file_name);
        std::fs::read_to_string(path)?
    };

    Ok(txt)
}

pub async fn load_binary(file_name: &str) -> anyhow::Result<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    let data = {
        let url = format_url(file_name);
        reqwest::get(url).await?.bytes().await?.to_vec()
    };
    #[cfg(not(target_arch = "wasm32"))]
    let data = {
        let path = std::path::Path::new(env!("OUT_DIR"))
            .join("res")
            .join(file_name);
        std::fs::read(path)?
    };

    Ok(data)
}
```

<Note>

Estamos usando `OUT_DIR` no desktop para acessar nossa pasta `res`.

</Note>

Estou usando a biblioteca [reqwest](https://docs.rs/reqwest) para gerenciar o carregamento das requisições ao usar WASM. Adicione o seguinte ao `Cargo.toml`:

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
# Outras dependências
reqwest = { version = "0.11" }
```

Também precisaremos adicionar a feature `Location` ao `web-sys`:

```toml
web-sys = { version = "0.3", features = [
    "Document",
    "Window",
    "Element",
    "Location",
]}
```

Certifique-se de adicionar `resources` como um módulo em `lib.rs`:

```rust
mod resources;
```

## Carregando modelos com TOBJ

Vamos usar a biblioteca [tobj](https://docs.rs/tobj/3.0/tobj/) para carregar nosso modelo. Vamos adicioná-la ao nosso `Cargo.toml`.

```toml
[dependencies]
# outras dependências...
tobj = { version = "3.2", default-features = false, features = ["async"]}
```

Antes de podermos carregar nosso modelo, porém, precisamos de um lugar para armazená-lo.

```rust
// model.rs
pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}
```

Você notará que nossa struct `Model` possui um `Vec` para `meshes` e `materials`. Isso é importante porque nosso arquivo `.obj` pode incluir múltiplas malhas (meshes) e materiais. Ainda precisamos criar as estruturas `Mesh` e `Material`, então vamos fazer isso.

```rust
pub struct Material {
    pub name: String,
    pub diffuse_texture: texture::Texture,
    pub bind_group: wgpu::BindGroup,
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}
```

O `Material` é bem simples. É apenas o nome e uma textura. Nosso obj de cubo na verdade possui duas texturas, mas uma é um normal map, e abordaremos isso [mais tarde](../../intermediate/tutorial11-normals). O nome serve mais para fins de depuração.

Falando em texturas, precisaremos adicionar uma função para carregar uma `Texture` em `resources.rs`.

```rust

pub async fn load_texture(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> anyhow::Result<texture::Texture> {
    let data = load_binary(file_name).await?;
    texture::Texture::from_bytes(device, queue, &data, file_name)
}
```

O método `load_texture` será útil quando carregarmos as texturas para nossos modelos, pois `include_bytes!` exige que saibamos o nome do arquivo em tempo de compilação, o que não podemos garantir com texturas de modelos.

`Mesh` contém um buffer de vértices, um buffer de índices e o número de índices na malha. Estamos usando um `usize` para o material. Esse `usize` indexará a lista `materials` quando chegar a hora de desenhar.

Com tudo isso resolvido, podemos ir para o carregamento do nosso modelo.

```rust
pub async fn load_model(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> anyhow::Result<model::Model> {
    let obj_text = load_string(file_name).await?;
    let obj_cursor = Cursor::new(obj_text);
    let mut obj_reader = BufReader::new(obj_cursor);

    let (models, obj_materials) = tobj::load_obj_buf_async(
        &mut obj_reader,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |p| async move {
            let mat_text = load_string(&p).await.unwrap();
            tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
        },
    )
    .await?;

    let mut materials = Vec::new();
    for m in obj_materials? {
        let diffuse_texture = load_texture(&m.diffuse_texture, device, queue).await?;
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
            ],
            label: None,
        });

        materials.push(model::Material {
            name: m.name,
            diffuse_texture,
            bind_group,
        })
    }

    let meshes = models
        .into_iter()
        .map(|m| {
                let vertices = (0..m.mesh.positions.len() / 3)
                .map(|i| {
                    if m.mesh.normals.is_empty(){
                        model::ModelVertex {
                            position: [
                                m.mesh.positions[i * 3],
                                m.mesh.positions[i * 3 + 1],
                                m.mesh.positions[i * 3 + 2],
                            ],
                            tex_coords: [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]],
                            normal: [0.0, 0.0, 0.0],
                        }
                    }else{
                        model::ModelVertex {
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
                        }
                    }
                })
                .collect::<Vec<_>>();

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

    Ok(model::Model { meshes, materials })
}
```

## Renderizando uma malha

Antes de podermos desenhar o modelo, precisamos ser capazes de desenhar uma malha individual. Vamos criar uma trait chamada `DrawModel` e implementá-la para o `RenderPass`.

```rust
// model.rs
pub trait DrawModel<'a> {
    fn draw_mesh(&mut self, mesh: &'a Mesh);
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        instances: Range<u32>,
    );
}
impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(&mut self, mesh: &'b Mesh) {
        self.draw_mesh_instanced(mesh, 0..1);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: Range<u32>,
    ){
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }
}
```

Poderíamos ter colocado esses métodos em um `impl Model`, mas achei que fazia mais sentido deixar o `RenderPass` cuidar de toda a renderização, já que essa é meio que a função dele. No entanto, isso significa que temos que importar o `DrawModel` quando formos renderizar.

Quando removemos `vertex_buffer`, etc., também removemos a configuração no render_pass.

```rust
// lib.rs
render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
render_pass.set_pipeline(&self.render_pipeline);
render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
render_pass.set_bind_group(1, &self.camera_bind_group, &[]);

use model::DrawModel;
render_pass.draw_mesh_instanced(&self.obj_model.meshes[0], 0..self.instances.len() as u32);
```

Antes disso, porém, precisamos carregar o modelo e salvá-lo no `State`. Coloque o seguinte em `State::new()`.

```rust
let obj_model =
    resources::load_model("cube.obj", &device, &queue, &texture_bind_group_layout)
        .await
        .unwrap();
```

Nosso novo modelo é um pouco maior que o anterior, então vamos precisar ajustar um pouco o espaçamento em nossas instâncias.

```rust
const SPACE_BETWEEN: f32 = 3.0;
let instances = (0..NUM_INSTANCES_PER_ROW).flat_map(|z| {
    (0..NUM_INSTANCES_PER_ROW).map(move |x| {
        let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
        let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

        let position = cgmath::Vector3 { x, y: 0.0, z };

        let rotation = if position.is_zero() {
            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
        } else {
            cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
        };

        Instance {
            position, rotation,
        }
    })
}).collect::<Vec<_>>();
```

Com tudo isso feito, você deverá obter algo como isto.

![cubes.png](./cubes.png)

## Usando as texturas corretas

Se você olhar para os arquivos de textura do nosso obj, verá que eles não correspondem ao nosso obj. A textura que queremos ver é esta,

![cube-diffuse.jpg](./cube-diffuse.jpg)

mas ainda estamos obtendo nossa textura da árvore feliz.

A razão para isso é bem simples. Embora tenhamos criado nossas texturas, não criamos um bind group para entregar ao `RenderPass`. Ainda estamos usando nosso antigo `diffuse_bind_group`. Se quisermos mudar isso, precisamos usar o bind group dos nossos materiais - o membro `bind_group` da struct `Material`.

Vamos adicionar um parâmetro de material ao `DrawModel`.

```rust
pub trait DrawModel<'a> {
    fn draw_mesh(&mut self, mesh: &'a Mesh, material: &'a Material, camera_bind_group: &'a wgpu::BindGroup);
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
    );

}

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(&mut self, mesh: &'b Mesh, material: &'b Material, camera_bind_group: &'b wgpu::BindGroup) {
        self.draw_mesh_instanced(mesh, material, 0..1, camera_bind_group);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, camera_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }
}
```

Precisamos alterar o código de renderização para refletir isso.

```rust
render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

render_pass.set_pipeline(&self.render_pipeline);

let mesh = &self.obj_model.meshes[0];
let material = &self.obj_model.materials[mesh.material];
render_pass.draw_mesh_instanced(mesh, material, 0..self.instances.len() as u32, &self.camera_bind_group);
```

Com tudo isso no lugar, devemos obter o seguinte.

![cubes-correct.png](./cubes-correct.png)

## Renderizando o modelo inteiro

No momento, estamos especificando a malha e o material diretamente. Isso é útil se quisermos desenhar uma malha com um material diferente. Também não estamos renderizando outras partes do modelo (se tivéssemos algumas). Vamos criar um método para `DrawModel` que desenhará todas as partes do modelo com seus respectivos materiais.

```rust
pub trait DrawModel<'a> {
    // ...
    fn draw_model(&mut self, model: &'a Model, camera_bind_group: &'a wgpu::BindGroup);
    fn draw_model_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a, {
    // ...
    fn draw_model(&mut self, model: &'b Model, camera_bind_group: &'b wgpu::BindGroup) {
        self.draw_model_instanced(model, 0..1, camera_bind_group);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(mesh, material, instances.clone(), camera_bind_group);
        }
    }
}
```

O código em `lib.rs` mudará de acordo.

```rust
render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
render_pass.set_pipeline(&self.render_pipeline);
render_pass.draw_model_instanced(&self.obj_model, 0..self.instances.len() as u32, &self.camera_bind_group);
```

## Demonstração

<WasmExample example="tutorial9_models"></WasmExample>

<AutoGithubLink/>
