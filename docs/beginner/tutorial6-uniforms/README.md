# Uniform buffers e uma câmera 3D

Embora todo o nosso trabalho anterior pareça ter sido em 2D, estivemos trabalhando em 3D o tempo todo! Essa é parte da razão pela qual nossa estrutura `Vertex` possui `position` como um array de 3 floats em vez de apenas 2. Não conseguimos ver o aspecto 3D da nossa cena porque a estamos observando de frente. Vamos mudar nosso ponto de vista criando uma `Camera`.

## Uma câmera de perspectiva

Este tutorial é mais sobre aprender a usar o wgpu e menos sobre álgebra linear, então vou passar rapidamente por grande parte da matemática envolvida. Há bastante material de leitura online se você estiver interessado no que acontece por baixo dos panos. Vamos usar a crate [cgmath](https://docs.rs/cgmath) para lidar com toda a matemática para nós. Adicione o seguinte ao seu `Cargo.toml`.

```toml
[dependencies]
# outras deps...
cgmath = "0.18"
```

Agora que temos uma biblioteca de matemática, vamos usá-la! Crie uma struct `Camera` acima da struct `State`.

```rust
struct Camera {
    eye: cgmath::Point3<f32>,
    target: cgmath::Point3<f32>,
    up: cgmath::Vector3<f32>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Camera {
    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        // 1.
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        // 2.
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);

        // 3.
        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
}
```

O método `build_view_projection_matrix` é onde a mágica acontece.
1. A matriz `view` move o mundo para ficar na posição e rotação da câmera. É essencialmente a inversa do que seria a matriz de transformação da câmera.
2. A matriz `proj` deforma a cena para criar o efeito de profundidade. Sem isso, objetos próximos teriam o mesmo tamanho de objetos distantes.
3. O sistema de coordenadas no Wgpu é baseado nos sistemas de coordenadas do DirectX e Metal. Isso significa que em [coordenadas normalizadas de dispositivo](https://github.com/gfx-rs/gfx/tree/master/src/backend/dx12#normalized-coordinates), os eixos x e y estão no intervalo de -1.0 a +1.0, e o eixo z está no intervalo de 0.0 a +1.0. A crate `cgmath` (assim como a maioria das crates de matemática para jogos) foi construída para o sistema de coordenadas do OpenGL. Esta matriz vai escalar e transladar nossa cena do sistema de coordenadas do OpenGL para o do WGPU. Vamos defini-la da seguinte forma.

```rust
#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::from_cols(
    cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 1.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 1.0),
);
```

* Nota: Não **precisamos** explicitamente da `OPENGL_TO_WGPU_MATRIX`, mas modelos centralizados em (0, 0, 0) ficarão pela metade dentro da área de clipping. Isso só é um problema se você não estiver usando uma matriz de câmera.

Agora vamos adicionar um campo `camera` à struct `State`.

```rust
pub struct State {
    // ...
    camera: Camera,
    // ...
}

async fn new(window: Window) -> Self {
    // let diffuse_bind_group ...

    let camera = Camera {
        // posiciona a câmera 1 unidade para cima e 2 unidades para trás
        // +z sai da tela
        eye: (0.0, 1.0, 2.0).into(),
        // faz ela olhar para a origem
        target: (0.0, 0.0, 0.0).into(),
        // qual direção é "para cima"
        up: cgmath::Vector3::unit_y(),
        aspect: config.width as f32 / config.height as f32,
        fovy: 45.0,
        znear: 0.1,
        zfar: 100.0,
    };

    Self {
        // ...
        camera,
        // ...
    }
}
```

Agora que temos nossa câmera e ela pode nos gerar uma matriz view projection, precisamos de um lugar para armazená-la. Também precisamos de uma forma de enviá-la para nossos shaders.

## O uniform buffer

Até este ponto, usamos `Buffer`s para armazenar nossos dados de vértices e índices, e até mesmo para carregar nossas texturas. Vamos usá-los novamente para criar o que é conhecido como um uniform buffer. Um uniform é um bloco de dados disponível para cada invocação de um conjunto de shaders. Tecnicamente, já usamos uniforms para nossa textura e sampler. Vamos usá-los novamente para armazenar nossa matriz view projection. Para começar, vamos criar uma struct para conter nosso uniform.

```rust
// Precisamos disso para que o Rust armazene nossos dados corretamente para os shaders
#[repr(C)]
// Isso é para podermos armazenar isso em um buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    // Não podemos usar cgmath com bytemuck diretamente, então teremos
    // que converter o Matrix4 em um array 4x4 de f32
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}
```

Agora que temos nossos dados estruturados, vamos criar nosso `camera_buffer`.

```rust
// em new() após criar `camera`

let mut camera_uniform = CameraUniform::new();
camera_uniform.update_view_proj(&camera);

let camera_buffer = device.create_buffer_init(
    &wgpu::util::BufferInitDescriptor {
        label: Some("Camera Buffer"),
        contents: bytemuck::cast_slice(&[camera_uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    }
);
```

## Uniform buffers e bind groups

Legal! Agora que temos um uniform buffer, o que fazemos com ele? A resposta é criar um bind group para ele. Primeiro, temos que criar o bind group layout.

```rust
let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    entries: &[
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    ],
    label: Some("camera_bind_group_layout"),
});
```

Alguns pontos a observar:

1. Definimos `visibility` como `ShaderStages::VERTEX`, pois só precisamos das informações da câmera no vertex shader, já que é ele que usaremos para manipular nossos vértices.
2. O `has_dynamic_offset` significa que a localização dos dados no buffer pode mudar. Esse será o caso se você armazenar múltiplos conjuntos de dados que variam de tamanho em um único buffer. Se você definir isso como true, terá que fornecer os offsets mais tarde.
3. `min_binding_size` especifica o menor tamanho que o buffer pode ter. Você não precisa especificar isso, então deixamos como `None`. Se quiser saber mais, pode consultar [a documentação](https://docs.rs/wgpu/latest/wgpu/enum.BindingType.html#variant.Buffer.field.min_binding_size).

Agora, podemos criar o bind group propriamente dito.

```rust
let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    layout: &camera_bind_group_layout,
    entries: &[
        wgpu::BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }
    ],
    label: Some("camera_bind_group"),
});
```

Assim como fizemos com nossa textura, precisamos registrar nosso `camera_bind_group_layout` no pipeline de renderização.

```rust
let render_pipeline_layout = device.create_pipeline_layout(
    &wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[
            Some(&texture_bind_group_layout),
            Some(&camera_bind_group_layout),
        ],
        immediate_size: 0,
    }
);
```

Agora precisamos adicionar `camera_buffer` e `camera_bind_group` ao `State`

```rust
pub struct State {
    // ...
    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
}

async fn new(window: Window) -> Self {
    // ...
    Self {
        // ...
        camera,
        camera_uniform,
        camera_buffer,
        camera_bind_group,
    }
}
```

A última coisa que precisamos fazer antes de ir para os shaders é usar o bind group em `render()`.

```rust
render_pass.set_pipeline(&self.render_pipeline);
render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
// NOVO!
render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
```

## Usando o uniform no vertex shader

Modifique o vertex shader para incluir o seguinte.

```wgsl
// Vertex shader
struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(1) @binding(0) // 1.
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0); // 2.
    return out;
}
```

1. Como criamos um novo bind group, precisamos especificar qual deles estamos usando no shader. O número é determinado pelo nosso `render_pipeline_layout`. O `texture_bind_group_layout` é listado primeiro, portanto é o `group(0)`, e o `camera_bind_group` é o segundo, portanto é o `group(1)`.
2. A ordem de multiplicação é importante quando se trata de matrizes. O vetor vai à direita, e as matrizes vão à esquerda em ordem de importância.

## Um controlador para nossa câmera

Se você executar o código agora, deverá obter algo como isto.

![./static-tree.png](./static-tree.png)

A forma agora está menos esticada, mas ainda está bastante estática. Você pode experimentar mover a posição da câmera por aí, mas a maioria das câmeras em jogos se move. Como este tutorial é sobre o uso do wgpu e não sobre como processar entradas do usuário, vou apenas postar o código do `CameraController` abaixo.

```rust
struct CameraController {
    speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
}

impl CameraController {
    fn new(speed: f32) -> Self {
        Self {
            speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
        }
    }

    fn handle_key(&mut self, code: KeyCode, is_pressed: bool) -> bool {
        match code {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.is_forward_pressed = is_pressed;
                true
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.is_left_pressed = is_pressed;
                true
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.is_backward_pressed = is_pressed;
                true
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.is_right_pressed = is_pressed;
                true
            }
            _ => false,
        }
    }

    fn update_camera(&self, camera: &mut Camera) {
        use cgmath::InnerSpace;
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();

        // Evita glitches quando a câmera fica muito próxima do
        // centro da cena.
        if self.is_forward_pressed && forward_mag > self.speed {
            camera.eye += forward_norm * self.speed;
        }
        if self.is_backward_pressed {
            camera.eye -= forward_norm * self.speed;
        }

        let right = forward_norm.cross(camera.up);

        // Refaz o cálculo do raio caso a tecla para frente/trás esteja pressionada.
        let forward = camera.target - camera.eye;
        let forward_mag = forward.magnitude();

        if self.is_right_pressed {
            // Redimensiona a distância entre o alvo e o olho para que não mude.
            // O olho, portanto, continua no círculo formado pelo alvo e pelo olho.
            camera.eye = camera.target - (forward + right * self.speed).normalize() * forward_mag;
        }
        if self.is_left_pressed {
            camera.eye = camera.target - (forward - right * self.speed).normalize() * forward_mag;
        }
    }
}
```

Este código não é perfeito. A câmera se move lentamente para trás quando você a rotaciona. No entanto, funciona para nossos propósitos. Sinta-se à vontade para melhorá-lo!

Ainda precisamos conectar isso ao nosso código existente para fazê-lo funcionar. Adicione o controlador ao `State` e crie-o em `new()`.

```rust
pub struct State {
    // ...
    camera: Camera,
    // NOVO!
    camera_controller: CameraController,
    // ...
}
// ...
impl State {
    async fn new(window: Arc<Window>) -> anyhow::Result<State> {
        // ...
        let camera_controller = CameraController::new(0.2);
        // ...

        Self {
            // ...
            camera_controller,
            // ...
        }
    }
}
```

Vamos atualizar o `camera_controller` na função `handle_key`.

```rust
    fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        if code == KeyCode::Escape && is_pressed {
            event_loop.exit();
        } else {
            self.camera_controller.handle_key(code, is_pressed);
        }
    }
```

Até este ponto, o controlador de câmera não está realmente fazendo nada. Os valores em nosso uniform buffer precisam ser atualizados. Existem alguns métodos principais para fazer isso.

1. Podemos criar um buffer separado e copiar seus conteúdos para o nosso `camera_buffer`. O novo buffer é conhecido como um staging buffer. Esse método é geralmente como é feito, pois permite que o conteúdo do buffer principal (neste caso, `camera_buffer`) seja acessível apenas pela GPU. A GPU pode fazer algumas otimizações de velocidade, o que não conseguiria se pudéssemos acessar o buffer via CPU.
2. Podemos chamar um dos métodos de mapeamento `map_read_async` e `map_write_async` no próprio buffer. Eles nos permitem acessar o conteúdo de um buffer diretamente, mas exigem que lidemos com o aspecto `async` desses métodos. Isso também exige que nosso buffer use `BufferUsages::MAP_READ` e/ou `BufferUsages::MAP_WRITE`. Não falaremos sobre isso aqui, mas confira o tutorial [Wgpu sem uma janela](../../showcase/windowless) se quiser saber mais.
3. Podemos usar `write_buffer` na `queue`.

Vamos usar a opção número 3.

```rust
fn update(&mut self) {
    self.camera_controller.update_camera(&mut self.camera);
    self.camera_uniform.update_view_proj(&self.camera);
    self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
}
```

Isso é tudo que precisamos fazer. Se você executar o código agora, deverá ver um pentágono com nossa textura de árvore que você pode rotacionar e aproximar/afastar usando as teclas WASD/setas.

## Demonstração

<WasmExample example="tutorial6_uniforms"></WasmExample>

<AutoGithubLink/>

## Desafio

Faça com que nosso modelo rotacione por conta própria, independentemente da câmera. *Dica: você precisará de outra matriz para isso.*
