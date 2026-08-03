# Mipmapping (Geração de Mipmaps)

## O que são Mipmaps?

Mipmaps são versões reduzidas de uma textura armazenadas na GPU em níveis (nível 0 = tamanho original, nível 1 = metade da resolução, nível 2 = 1/4 da resolução, etc.). Quando um objeto está distante da câmera, a GPU seleciona o nível de mip adequado para evitar serrilhamento e ruído visual na distância.

![Superfície sem mipmaps](./ground-with-no-mips.png)
![Superfície com mipmaps](./ground-with-mips.png)

## Como Gerar Mipmaps em Wgpu

Existem duas formas principais demonstradas neste projeto:

1. **Geração via Render Pass (Blitting)**:
   Desenha progressivamente cada nível de mip (`mip N` -> `mip N+1`) usando amostragem linear (`wgpu::FilterMode::Linear`) em um RenderPass.

2. **Geração via Compute Shader**:
   Usa um Compute Shader para carregar blocos de pixels 2x2 do nível anterior, tirar a média e gravar no nível seguinte via `storage_texture_2d`.

<AutoGithubLink/>
