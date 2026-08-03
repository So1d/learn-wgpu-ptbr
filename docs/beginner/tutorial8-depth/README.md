# O Depth Buffer

Vamos dar uma olhada mais de perto no último exemplo de um ângulo diferente.

![depth_problems.png](./depth_problems.png)

Modelos que deveriam estar atrás estão sendo renderizados à frente daqueles na frente. Isso é causado pela ordem de desenho (draw order). Por padrão, os dados de pixel de um novo objeto substituirão os dados de pixel antigos.

Existem duas maneiras de resolver isso: ordenar os dados de trás para frente ou usar o que é conhecido como depth buffer (buffer de profundidade).

## Ordenando de trás para frente

Este é o método padrão para renderização 2D, pois é bastante fácil saber o que deve ficar na frente do quê. Você pode apenas usar a ordem z (z-order). Na renderização 3D, isso fica um pouco mais complicado porque a ordem dos objetos muda com base no ângulo da câmera.

Uma forma simples de fazer isso é ordenar todos os objetos pela distância em relação à posição da câmera. No entanto, há falhas nesse método: quando um objeto grande está atrás de um objeto pequeno, partes do objeto grande que deveriam estar à frente do objeto pequeno serão renderizadas atrás dele. Também enfrentaremos problemas com objetos que se sobrepõem a *si mesmos*.

Se quisermos fazer isso corretamente, precisamos ter precisão ao nível de pixel. É aí que entra o *depth buffer*.

## A profundidade de um pixel

Um depth buffer é uma textura em escala de cinza (preto e branco) que armazena a coordenada z dos pixels renderizados. O Wgpu pode usar isso ao desenhar novos pixels para determinar se deve substituir ou manter os dados. Essa técnica é chamada de teste de profundidade (depth testing). Isso resolverá nosso problema de ordem de desenho sem a necessidade de ordenar nossos objetos!

Vamos criar uma função para criar a textura de profundidade em `texture.rs`.

```rust
impl Texture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float; // 1.
    
    pub fn create_depth_texture(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, label: &str) -> Self {
        let size = wgpu::Extent3d { // 2.
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT // 3.
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(
            &wgpu::SamplerDescriptor { // 4.
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                compare: Some(wgpu::CompareFunction::LessEqual), // 5.
                lod_min_clamp: 0.0,
                lod_max_clamp: 100.0,
                ..Default::default()
            }
        );

        Self { texture, view, sampler }
    }
}
```

1. Precisamos do DEPTH_FORMAT para criar o estágio de profundidade do `render_pipeline` e para criar a própria textura de profundidade.
2. Nossa textura de profundidade precisa ter o mesmo tamanho da nossa tela se quisermos que as coisas sejam renderizadas corretamente. Podemos usar nosso `config` para garantir que nossa textura de profundidade tenha o mesmo tamanho das nossas texturas de superfície.
3. Como estamos renderizando para esta textura, precisamos adicionar a flag `RENDER_ATTACHMENT` a ela.
4. Tecnicamente não *precisamos* de um sampler para uma textura de profundidade, mas nossa struct `Texture` o exige, e precisaremos de um se algum dia quisermos amostrá-la.
5. Se decidirmos renderizar nossa textura de profundidade, precisamos usar `CompareFunction::LessEqual`. Isso se deve a como a `sampler_comparison` e a `textureSampleCompare()` interagem com a função `texture()` no GLSL.

Criamos nossa `depth_texture` em `State::new()`.

```rust
let depth_texture = texture::Texture::create_depth_texture(&device, &config, "depth_texture");
```

Precisamos modificar nosso `render_pipeline` para permitir o teste de profundidade.

```rust
let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    // ...
    depth_stencil: Some(wgpu::DepthStencilState {
        format: texture::Texture::DEPTH_FORMAT,
        depth_write_enabled: Some(true),
        depth_compare: Some(wgpu::CompareFunction::Less), // 1.
        stencil: wgpu::StencilState::default(), // 2.
        bias: wgpu::DepthBiasState::default(),
    }),
    // ...
});
```

1. A função `depth_compare` nos diz quando descartar um novo pixel. Usar `LESS` significa que os pixels serão desenhados da frente para trás. Aqui estão os outros valores possíveis para uma [CompareFunction](https://docs.rs/wgpu/latest/wgpu/enum.CompareFunction.html) que você pode usar:

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CompareFunction {
    Undefined = 0,
    Never = 1,
    Less = 2,
    Equal = 3,
    LessEqual = 4,
    Greater = 5,
    NotEqual = 6,
    GreaterEqual = 7,
    Always = 8,
}
```

2. Existe outro tipo de buffer chamado stencil buffer. É uma prática comum armazenar o stencil buffer e o depth buffer na mesma textura. Estes campos controlam os valores para o teste de stencil. Usaremos valores padrão, pois não estamos usando um stencil buffer. Cobriremos stencil buffers [mais tarde](../../todo).

Não se esqueça de armazenar a `depth_texture` no `State`.

```rust
pub struct State {
    // ...
    depth_texture: Texture,
    // ...
}

async fn new(window: Window) -> Self {
    // ...
    
    Self {
        // ...
        depth_texture,
        // ...
    }
}
```

Precisamos lembrar de alterar o método `resize()` para criar uma nova `depth_texture` e `depth_texture_view`.

```rust
fn resize(&mut self, width: u32, height: u32) {
    // ...

    self.depth_texture = texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");

    // ...
}
```

Certifique-se de atualizar a `depth_texture` *depois* de atualizar o `config`. Se não o fizer, seu programa falhará, pois a `depth_texture` terá um tamanho diferente da textura da `surface`.

A última alteração que precisamos fazer é na função `render()`. Criamos a `depth_texture`, mas atualmente não a estamos usando. Nós a usamos anexando-a ao `depth_stencil_attachment` de um render pass.

```rust
let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
    // ...
    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
        view: &self.depth_texture.view,
        depth_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(1.0),
            store: wgpu::StoreOp::Store,
        }),
        stencil_ops: None,
    }),
});
```

E isso é tudo o que temos que fazer! Nenhum código de shader é necessário! Se você executar a aplicação, os problemas de profundidade estarão corrigidos.

![forest_fixed.png](./forest_fixed.png)

## Demonstração

<WasmExample example="tutorial8_depth"></WasmExample>

<AutoGithubLink/>

## Desafio

Como o depth buffer é uma textura, podemos amostrá-lo no shader.
Crie um bind group para a textura de profundidade (or reutilize um existente),
e renderize-o na tela.
