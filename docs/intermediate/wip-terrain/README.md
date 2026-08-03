# Terreno Procedural

## Visão Geral

Neste tutorial em rascunho (WIP), geramos malhas de terreno procedural diretamente na GPU usando **Compute Shaders** com funções de ruído Simplex (OpenSimplex) combinadas com **Fractal Brownian Motion (FBM)**.

## Compute Shaders e Ruído Simplex

O Compute Shader calcula os vértices e índices da malha em paralelo na GPU.

### Exemplo de função WGSL (`terrain.wgsl`):

```wgsl
fn fbm(p: vec2<f32>) -> f32 {
    let NUM_OCTAVES: u32 = 5u;
    var x = p * 0.01;
    var v = 0.0;
    var a = 0.5;
    let shift = vec2<f32>(100.0);
    let cs = vec2<f32>(cos(0.5), sin(0.5));
    let rot = mat2x2<f32>(cs.x, cs.y, -cs.y, cs.x);

    for (var i=0u; i<NUM_OCTAVES; i=i+1u) {
        v = v + a * snoise2(x);
        x = rot * x * 2.0 + shift;
        a = a * 0.5;
    }

    return v;
}
```

![Terreno com FBM aplicado](./figure_fbm.png)

## Workgroups na GPU

O Compute Shader organiza o processamento em **Workgroups** de tamanho fixo (ex: `@workgroup_size(64)`), onde cada thread processa um vértice ou quad do terreno.

![Grid de Workgroups](./figure_work-groups.jpg)