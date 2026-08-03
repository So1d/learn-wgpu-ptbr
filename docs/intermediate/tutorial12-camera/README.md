# Uma Câmera Melhor (Câmera 3D FPS)

## Visão Geral

Nesta seção, desacoplamos o gerenciamento de visão e projeção criando uma struct `Camera`, uma struct `Projection` e um controlador interativo `CameraController`.

## A Câmera e a Projeção

Em um arquivo `camera.rs`:

```rust
use cgmath::*;
use winit::event::*;
use winit::dpi::PhysicalPosition;
use instant::Duration;
use std::f32::consts::FRAC_PI_2;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::from_cols(
    cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 1.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 1.0),
);

pub struct Camera {
    pub position: Point3<f32>,
    yaw: Rad<f32>,
    pitch: Rad<f32>,
}

pub struct Projection {
    aspect: f32,
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
}
```

A matriz de visão (`view matrix`) é calculada usando as coordenadas esféricas `yaw` e `pitch`, e a matriz de projeção (`projection matrix`) é atualizada sempre que a janela muda de dimensão.

## O Controlador de Câmera (CameraController)

Processa comandos via WASD, Shift/Espaço para movimentação vertical, e rastreia o movimento do cursor do mouse (`DeviceEvent::MouseMotion`) para rotação estilo jogos FPS.

```rust
pub struct CameraController {
    // ...
    speed: f32,
    sensitivity: f32,
}
```

## Delta Time (`dt`)

Calculamos o tempo decorrido entre quadros (`dt = instant::Instant::now() - last_render_time`) para garantir que a movimentação da câmera e rotações da cena permaneçam suaves independente da taxa de quadros (FPS) do dispositivo.

![Captura de tela do modelo 3D com câmera FPS](./screenshot.png)

## Demonstração

<WasmExample example="tutorial12_camera"></WasmExample>

<AutoGithubLink/>
