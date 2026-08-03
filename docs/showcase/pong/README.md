# Pong

![Screenshot do jogo Pong](./pong.png)

<Note class="warning">

Este exemplo não está funcionando a partir do `wgpu = "28.0"`. Se o crate atualizar para a versão mais recente, eu atualizarei o código, mas como o mantenedor do crate está direcionando os usuários para usar o [glyphon](https://github.com/grovesNL/glyphon?tab=readme-ov-file), estou considerando migrar para ele ou escrever meu próprio código de renderização de texto.

</Note>

Praticamente o "Hello World!" do desenvolvimento de jogos. Pong foi recriado milhares de vezes. Eu conheço o Pong, você conhece o Pong, todos conhecemos o Pong. Dito isso, desta vez eu quis colocar um pouco mais de esforço do que a maioria das pessoas costuma colocar. Esta demonstração (showcase) inclui um sistema de menu básico, efeitos sonoros e diferentes estados de jogo.

A arquitetura não é perfeita, pois segui a mentalidade de "fazer funcionar". Se eu fosse refazer este projeto, mudaria muitas coisas. De qualquer forma, vamos à análise pós-morte (postmortem).

## A Arquitetura

Estava experimentando separar o estado da lógica de renderização. O resultado final ficou semelhante a um modelo de Entity Component System (ECS).

Criei uma struct `State` com todos os objetos da cena. Isso incluiu a bola e as raquetes, bem como os textos para a pontuação e até o menu. `State` também inclui um campo `game_state` do tipo `GameState`.

```rust
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum GameState {
    MainMenu,
    Serving,
    Playing,
    GameOver,
    Quiting,
}
```

A struct `State` não possui métodos próprios, pois adotei uma abordagem orientada a dados. Em vez disso, criei uma trait `System` e implementei várias structs que a implementam.

```rust
pub trait System {
    #[allow(unused_variables)]
    fn start(&mut self, state: &mut state::State) {}
    fn update_state(
        &self, 
        input: &input::Input, 
        state: &mut state::State, 
        events: &mut Vec<state::Event>,
    );
}
```

Os sistemas são responsáveis por controlar e atualizar o estado dos diferentes objetos (posição, visibilidade, etc.), além de atualizar o campo `game_state`. Criei todos os sistemas na inicialização e usei um `match` em `game_state` para determinar quais deveriam ser executados em cada momento (o `visiblity_system` sempre é executado, pois é necessário em todos os estados).

```rust
visiblity_system.update_state(&input, &mut state, &mut events);
match state.game_state {
    state::GameState::MainMenu => {
        menu_system.update_state(&input, &mut state, &mut events);
        if state.game_state == state::GameState::Serving {
            serving_system.start(&mut state);
        }
    },
    state::GameState::Serving => {
        serving_system.update_state(&input, &mut state, &mut events);
        play_system.update_state(&input, &mut state, &mut events);
        if state.game_state == state::GameState::Playing {
            play_system.start(&mut state);
        }
    },
    state::GameState::Playing => {
        ball_system.update_state(&input, &mut state, &mut events);
        play_system.update_state(&input, &mut state, &mut events);
        if state.game_state == state::GameState::Serving {
            serving_system.start(&mut state);
        } else if state.game_state == state::GameState::GameOver {
            game_over_system.start(&mut state);
        }
    },
    state::GameState::GameOver => {
        game_over_system.update_state(&input, &mut state, &mut events);
        if state.game_state == state::GameState::MainMenu {
            menu_system.start(&mut state);
        }
    },
    state::GameState::Quiting => {},
}
```

Definitivamente não é o código mais limpo, mas funciona.

Acabei tendo 6 sistemas no total:

1. Adicionei o `VisibilitySystem` perto do fim do desenvolvimento. Até aquele momento, todos os sistemas tinham que definir o campo `visible` dos objetos. Isso era entediante e poluía a lógica. Então decidi criar o `VisibilitySystem` para tratar disso.

2. O `MenuSystem` controlava qual texto do menu estava em foco e o que acontecia quando o usuário pressionava a tecla Enter. Se o botão `Play` estivesse em foco, pressionar Enter mudava o `game_state` para `GameState::Serving`, iniciando o jogo. O botão `Quit` alternava para `GameState::Quiting`.

3. O `ServingSystem` define a posição da bola em `(0.0, 0.0)`, atualiza os textos de pontuação e altera para `GameState::Playing` após um temporizador.

4. O `PlaySystem` controla os jogadores. Permite que eles se movam e os impede de sair do espaço de jogo. Este sistema roda tanto em `GameState::Playing` quanto em `GameState::Serving`. Fiz isso para permitir que os jogadores se reposicionem antes do saque. O `PlaySystem` também altera para `GameState::GameOver` quando a pontuação de um dos jogadores é maior que 2.

5. O `BallSystem` controla o movimento da bola, bem como suas colisões/quiques nas paredes e jogadores. Ele também atualiza a pontuação e muda para `GameState::Serving` quando a bola sai pela lateral da tela.

6. O `GameOver` system atualiza o `win_text` e muda para `GameState::MainMenu` após um atraso.

Achei a abordagem baseada em sistemas bastante agradável de trabalhar. Minha implementação não foi a melhor, mas gostaria de trabalhar com isso novamente no futuro. Talvez eu até implemente meu próprio ECS.

## Entrada de Dados (Input)

A trait `System` originalmente possuía um método `process_input`. Isso se tornou um problema quando estava implementando a movimentação dos jogadores entre os saques. Os jogadores ficavam travados quando o `game_state` alternava de `Serving` para `Playing` porque as entradas de dados ficavam presas. Eu só chamava `process_input` em sistemas que estavam ativos no momento. Alterar isso seria delicado, então decidi mover todo o código de input para sua própria struct.

```rust
use winit::event::{VirtualKeyCode, ElementState};

#[derive(Debug, Default)]
pub struct Input {
    pub p1_up_pressed: bool,
    pub p1_down_pressed: bool,
    pub p2_up_pressed: bool,
    pub p2_down_pressed: bool,
    pub enter_pressed: bool,
}

impl Input {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn update(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        let pressed = state == ElementState::Pressed;
        match key {
            VirtualKeyCode::Up => {
                self.p2_up_pressed = pressed;
                true
            }
            VirtualKeyCode::Down => {
                self.p2_down_pressed = pressed;
                true
            }
            VirtualKeyCode::W => {
                self.p1_up_pressed = pressed;
                true
            }
            VirtualKeyCode::S => {
                self.p1_down_pressed = pressed;
                true
            }
            VirtualKeyCode::Return => {
                self.enter_pressed = pressed;
                true
            }
            _ => false
        }
    }

    pub fn ui_up_pressed(&self) -> bool {
        self.p1_up_pressed || self.p2_up_pressed
    }

    pub fn ui_down_pressed(&self) -> bool {
        self.p1_down_pressed || self.p2_down_pressed
    }
}
```

Isso funciona muito bem. Eu simplesmente passo essa struct no método `update_state`.

## Renderização (Render)

Usei [wgpu_glyph](https://docs.rs/wgpu_glyph) para os textos e quads brancos para a bola e as raquetes. Não há muito o que comentar aqui, é Pong afinal.

No entanto, experimentei usar batching (agrupamento de chamadas de desenho). Foi totalmente exagerado para este projeto, mas foi uma boa experiência de aprendizado. Aqui está o código se tiver interesse:

```rust
pub struct QuadBufferBuilder {
    vertex_data: Vec<Vertex>,
    index_data: Vec<u32>,
    current_quad: u32,
}

impl QuadBufferBuilder {
    pub fn new() -> Self {
        Self {
            vertex_data: Vec::new(),
            index_data: Vec::new(),
            current_quad: 0,
        }
    }

    pub fn push_ball(self, ball: &state::Ball) -> Self {
        if ball.visible {
            let min_x = ball.position.x - ball.radius;
            let min_y = ball.position.y - ball.radius;
            let max_x = ball.position.x + ball.radius;
            let max_y = ball.position.y + ball.radius;
    
            self.push_quad(min_x, min_y, max_x, max_y)
        } else {
            self
        }
    }

    pub fn push_player(self, player: &state::Player) -> Self {
        if player.visible {
            self.push_quad(
                player.position.x - player.size.x * 0.5, 
                player.position.y - player.size.y * 0.5, 
                player.position.x + player.size.x * 0.5,
                player.position.y + player.size.y * 0.5, 
            )
        } else {
            self
        }
    }

    pub fn push_quad(mut self, min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self {
        self.vertex_data.extend(&[
            Vertex {
                position: (min_x, min_y).into(),
            },
            Vertex {
                position: (max_x, min_y).into(),
            },
            Vertex {
                position: (max_x, max_y).into(),
            },
            Vertex {
                position: (min_x, max_y).into(),
            },
        ]);
        self.index_data.extend(&[
            self.current_quad * 4 + 0,
            self.current_quad * 4 + 1,
            self.current_quad * 4 + 2,
            self.current_quad * 4 + 0,
            self.current_quad * 4 + 2,
            self.current_quad * 4 + 3,
        ]);
        self.current_quad += 1;
        self
    }

    pub fn build(self, device: &wgpu::Device) -> (StagingBuffer, StagingBuffer, u32) {
        (
            StagingBuffer::new(device, &self.vertex_data),
            StagingBuffer::new(device, &self.index_data),
            self.index_data.len() as u32,
        )
    }
}
```

## Som

Usei [rodio](https://docs.rs/rodio) para o áudio. Criei uma struct `SoundPack` para armazenar os sons. Decidir como reproduzir os sons exigiu alguma reflexão. Optei por passar um `Vec<state::Event>` no método `update_state`. O sistema insere um evento no `Vec`. O enum `Event` é exibido abaixo.

```rust
#[derive(Debug, Copy, Clone)]
pub enum Event {
    ButtonPressed,
    FocusChanged,
    BallBounce(cgmath::Vector2<f32>),
    Score(u32),
}
```

Eu pretendia fazer o `BallBounce` reproduzir um som posicionado no espaço usando `SpatialSink`, mas enfrentei problemas de clipping (corte de áudio) e queria concluir o projeto. Fora isso, o sistema de eventos funcionou muito bem.

## Suporte a WASM

Este exemplo funciona na web, mas houve alguns passos necessários para fazer tudo funcionar. O primeiro foi alternar para um `lib.rs` em vez de utilizar apenas `main.rs`. Optei por usar [wasm-pack](https://rustwasm.github.io/wasm-pack/) para gerar o WebAssembly. Eu poderia ter mantido o formato anterior usando wasm-bindgen diretamente, mas enfrentei conflitos de versão do wasm-bindgen, então decidi optar pelo wasm-pack.

Para que o wasm-pack funcione corretamente, primeiro precisei adicionar algumas dependências:

```toml
[dependencies]
anyhow = "1.0"
env_logger = "0.10"
winit = { version = "0.30", features = ["android-native-activity"] }
anyhow = "1.0"
bytemuck = { version = "1.24", features = [ "derive" ] }
cgmath = "0.18"
pollster = "0.3"
wgpu = { version = "28.0", features = ["spirv"]}
wgpu_glyph = "0.19"
rand = "0.8"
rodio = { version = "0.15", default-features = false, features = ["wav"] }
log = "0.4"
instant = "0.1"

[target.'cfg(target_arch = "wasm32")'.dependencies]
console_error_panic_hook = "0.1.6"
console_log = "1.0"
getrandom = { version = "0.2", features = ["js"] }
rodio = { version = "0.15", default-features = false, features = ["wasm-bindgen", "wav"] }
wasm-bindgen-futures = "0.4.20"
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = [
    "Document",
    "Window",
    "Element",
]}
wgpu = { version = "28.0", features = ["spirv", "webgl"]}

[build-dependencies]
anyhow = "1.0"
fs_extra = "1.2"
glob = "0.3"
rayon = "1.4"
naga = { version = "28.0", features = ["glsl-in", "spv-out", "wgsl-out"]}
```

Destaco algumas delas:

- rand: Se quiser usar rand na web, você precisa incluir getrandom diretamente e habilitar sua feature `js`.
- rodio: Tive que desativar todas as features para a compilação WASM e depois reabilitá-las separadamente. A feature `mp3` especificamente não estava funcionando. Poderia haver uma solução alternativa, mas como não utilizo mp3 neste exemplo, optei por usar apenas wav.
- instant: Este crate é basicamente um wrapper em torno de `std::time::Instant`. Em uma compilação nativa, é apenas um alias de tipo. Em compilações para web, ele usa as funções de tempo do navegador.
- cfg-if: Um crate conveniente para tornar código específico de plataforma menos incômodo de escrever.
- env_logger e console_log: O env_logger não funciona em WebAssembly, por isso precisamos usar um logger diferente. O console_log é o utilizado nos tutoriais de WebAssembly, então optei por ele.
- wasm-bindgen: Este crate é a ponte que faz o código Rust funcionar na web. Se você estiver compilando usando o comando wasm-bindgen, certifique-se de que a versão da ferramenta corresponda **exatamente** à versão no Cargo.toml, caso contrário enfrentará erros. Se usar wasm-pack, ele baixará o binário do wasm-bindgen apropriado para o seu crate.
- web-sys: Contém funções e tipos que permitem usar métodos disponíveis em JS, tais como "getElementById()".

Agora que esclarecemos isso, vamos falar de código. Primeiro, precisamos criar uma função que iniciará nosso event loop.

```rust
#[cfg(target_arch="wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch="wasm32", wasm_bindgen(start))]
pub fn start() {
    // Omitting...
}
```

O `wasm_bindgen(start)` indica ao wasm-bindgen que esta função deve ser executada assim que o módulo WebAssembly for carregado pelo JavaScript. A maior parte do código dentro desta função é semelhante ao encontrado em outros exemplos deste site, mas há coisas específicas que precisamos fazer na web.

```rust
cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        console_log::init_with_level(log::Level::Warn).expect("Couldn't initialize logger");
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    } else {
        env_logger::init();
    }
}
```

Este código deve ser executado antes de qualquer outra tarefa significativa. Ele configura o logger com base na arquitetura alvo da compilação. A maioria das arquiteturas usará `env_logger`. A arquitetura `wasm32` usará `console_log`. Também é importante instruir o Rust a redirecionar panics para o JavaScript. Se não fizéssemos isso, não teríamos como saber quando nosso código Rust entra em panic.

Em seguida, criamos uma janela. A maior parte é como fizemos antes, mas como suportamos tela cheia, precisamos de algumas etapas extras:

```rust
let event_loop = EventLoop::new();
let monitor = event_loop.primary_monitor().unwrap();
let video_mode = monitor.video_modes().next();
let size = video_mode.clone().map_or(PhysicalSize::new(800, 600), |vm| vm.size());
let window = WindowBuilder::new()
    .with_visible(false)
    .with_title("Pong")
    .with_fullscreen(video_mode.map(|vm| Fullscreen::Exclusive(vm)))
    .build(&event_loop)
    .unwrap();

// Compilações WASM não têm acesso às informações do monitor, então
// devemos especificar uma resolução de fallback
if window.fullscreen().is_none() {
    window.set_inner_size(PhysicalSize::new(512, 512));
}
```

Em seguida, realizamos alguns passos específicos de web caso estejamos nessa plataforma:

```rust
#[cfg(target_arch = "wasm32")]
{
    use winit::platform::web::WindowExtWebSys;
    web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| {
            let dst = doc.get_element_by_id("wasm-example")?;
            let canvas = web_sys::Element::from(window.canvas()?);
            dst.append_child(&canvas).ok()?;

            // Solicita tela cheia; se negado, continua normalmente
            match canvas.request_fullscreen() {
                Ok(_) => {},
                Err(_) => ()
            }

            Some(())
        })
        .expect("Couldn't append canvas to document body.");
}
```

Todo o restante funciona da mesma maneira.

## Resumo

Um projeto divertido de desenvolver. Foi ultra-arquitetado e um pouco difícil de realizar alterações, mas ainda assim uma ótima experiência de aprendizado.

<!-- Experimente o código abaixo! (Controles atualmente requerem teclado.)

<WasmExample example="pong"></WasmExample> -->