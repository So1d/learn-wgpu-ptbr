# Dependências e a Janela

## Um pouco de preparação

Alguns de vocês lendo este tutorial já possuem muita experiência com a criação de janelas em Rust e provavelmente têm sua biblioteca favorita. No entanto, como este guia foi feito para todos, este é um tópico essencial que precisamos cobrir. Se você já sabe como gerenciar janelas em Rust, pode pular esta parte. O único detalhe fundamental é que a solução de janelas escolhida deve suportar a crate [raw-window-handle](https://github.com/rust-windowing/raw-window-handle).

## Quais crates estamos utilizando?

Para a parte iniciante, vamos manter as coisas o mais simples possível. Adicionaremos novas dependências à medida que avançarmos, mas abaixo estão as seções relevantes do `Cargo.toml` inicial:

```toml
[dependencies]
anyhow = "1.0"
winit = { version = "0.30", features = ["android-native-activity"] }
env_logger = "0.10"
log = "0.4"
wgpu = "30.0"
pollster = "0.3"
```

## Utilizando o novo resolver do Rust

A partir da versão 0.10, o wgpu exige o [novo resolver de features do Cargo](https://doc.rust-lang.org/cargo/reference/resolver.html#feature-resolver-version-2), que é o padrão no edition 2021 (qualquer novo projeto iniciado com Rust 1.56.0 ou superior). No entanto, se você ainda estiver utilizando a edição 2018, precisará incluir `resolver = "2"` na seção `[package]` do seu `Cargo.toml` (se for um crate único) ou na seção `[workspace]` da raiz do projeto.

## env_logger

É extremamente importante ativar os logs chamando `env_logger::init();`.
Quando o wgpu encontra qualquer erro interno, ele lança um panic com uma mensagem genérica enquanto envia a mensagem de erro real através da crate `log`.
Isso significa que, se você não chamar `env_logger::init()`, o wgpu falhará silenciosamente, deixando você sem entender o motivo do problema! (Já ativamos isso no código abaixo).

## Criando um novo projeto

Execute ```cargo new nome_do_projeto``` onde `nome_do_projeto` é o nome desejado.  
(No exemplo abaixo, utilizamos 'tutorial1_window').

## O código

Precisamos de uma estrutura para armazenar todo o nosso estado, então vamos criar uma struct chamada `State`.

```rust
use std::sync::Arc;

use winit::{
    application::ApplicationHandler, event::*, event_loop::{ActiveEventLoop, EventLoop}, keyboard::{KeyCode, PhysicalKey}, window::Window
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::EventLoopExtWebSys;

// Esta struct armazenará o estado da nossa aplicação/jogo
pub struct State {
    window: Arc<Window>,
}

impl State {
    // Não precisamos que isto seja assíncrono agora,
    // mas precisaremos no próximo tutorial
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        Ok(Self {
            window,
        })
    }

    pub fn resize(&mut self, _width: u32, _height: u32) {
        // Faremos alterações aqui no próximo tutorial
    }
    
    pub fn render(&mut self) {
        self.window.request_redraw();

        // Faremos o render real aqui no próximo tutorial
    }
}
```

Não há muita coisa acontecendo aqui ainda, mas assim que começarmos a usar o WGPU, esta struct se preencherá rapidamente. A maioria dos métodos nesta struct são temporários, embora no `render()` nós já solicitemos à janela para redesenhar um novo frame assim que possível, pois a `winit` só redesenha quadros quando a janela é redimensionada ou quando solicitamos explicitamente.

Agora que temos nossa struct `State`, precisamos dizer à winit como utilizá-la. Criaremos uma struct `App` para isso.

```rust
pub struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<State>,
}

impl App {
    pub fn new(#[cfg(target_arch = "wasm32")] event_loop: &EventLoop<State>) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());
        Self {
            state: None,
            #[cfg(target_arch = "wasm32")]
            proxy,
        }
    }
}
```

A struct `App` possui dois campos: `state` e `proxy`.

A variável `state` armazena nossa struct `State` como um `Option`. O motivo de usarmos um `Option` é que `State::new()` necessita de uma janela, e não podemos criar uma janela até que a aplicação alcance o estado `Resumed`.

A variável `proxy` só é necessária na Web, pois a criação de recursos do WGPU é um processo assíncrono.

Com a struct `App` criada, implementamos a trait `ApplicationHandler`. Ela nos oferece vários métodos para tratar eventos da aplicação, como pressionamento de teclas, movimento de mouse e eventos do ciclo de vida. Começaremos pelos métodos `resumed` e `user_event`:

```rust
impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;
            
            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
        }

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            // No ambiente nativo, usamos o pollster para aguardar a criação do State
            self.state = Some(pollster::block_on(State::new(window)).unwrap());
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Na Web, rodamos a future assincronamente e enviamos o resultado
            // para o event loop usando o proxy
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(proxy
                        .send_event(
                            State::new(window)
                                .await
                                .expect("Não foi possível criar o canvas!!!")
                        )
                        .is_ok())
                });
            }
        }
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        // É aqui que o proxy.send_event() chega na Web
        #[cfg(target_arch = "wasm32")]
        {
            event.window.request_redraw();
            event.resize(
                event.window.inner_size().width,
                event.window.inner_size().height,
            );
        }
        self.state = Some(event);
    }

    // ...
}
```

O método `resumed` faz o seguinte:
- Define atributos da janela (incluindo configurações específicas para Web se aplicável).
- Cria a janela com esses atributos.
- Cria uma future para instanciar a nossa struct `State`.
- No desktop (nativo), usa o `pollster` para aguardar a inicialização.
- Na Web, executa a future assincronamente e envia o resultado para a função `user_event`.

Em seguida, implementamos o `window_event`:

```rust
impl ApplicationHandler<State> for App {

    // ...

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                state.render();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => match (code, key_state.is_pressed()) {
                (KeyCode::Escape, true) => event_loop.exit(),
                _ => {}
            },
            _ => {}
        }
    }
}
```

Aqui processamos entradas de teclado, redimensionamento da janela e solicitações de renderização.

Por fim, criamos a função `run()` para rodar a aplicação:

```rust
pub fn run() -> anyhow::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_log::init_with_level(log::Level::Info).unwrap_throw();
    }

    let event_loop = EventLoop::with_user_event().build()?;
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut app = App::new();
        event_loop.run_app(&mut app)?;
    }
    #[cfg(target_arch = "wasm32")]
    {
        let app = App::new(&event_loop);
        event_loop.spawn_app(app);
    }

    Ok(())
}
```

## Suporte para Web (WASM)

Para rodar a aplicação no navegador via WebAssembly, ajustamos o `Cargo.toml`:

```toml
[lib]
crate-type = ["cdylib", "rlib"]
```

Isso permite gerar tanto uma biblioteca nativa Rust (`rlib`) quanto a biblioteca compatível com C/WebAssembly (`cdylib`).

<Note>

## WebAssembly (WASM)

O WebAssembly é um formato binário suportado por navegadores modernos que permite executar código compilado de linguagens como Rust na Web com alto desempenho.

</Note>

## Demonstração

Clique no botão abaixo para ver o código em execução:

<WasmExample example="tutorial1_window"></WasmExample>

<AutoGithubLink/>
