# Multithreading com Wgpu

O `wgpu` foi projetado para ser thread-safe (`Send` e `Sync`). É possível dividir o trabalho de renderização em múltiplas threads no Rust:
- Gravar encoders de comandos em threads separadas.
- Enviar os command buffers resultantes para a mesma `wgpu::Queue`.

<AutoGithubLink/>