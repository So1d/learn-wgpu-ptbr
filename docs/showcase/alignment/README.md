# Alinhamento de Memória (Memory Alignment)

Ao enviar dados em Uniform Buffers e Storage Buffers, a especificação WGSL exige regras de alinhamento rígidas para tipos como `vec3` (alinhado em 16 bytes) e matrizes. O descumprimento causa erros de validação ou comportamentos indefinidos no shader.

<AutoGithubLink/>