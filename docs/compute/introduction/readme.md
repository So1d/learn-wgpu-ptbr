# Introdução aos Compute Pipelines

Compute pipelines são um dos recursos mais empolgantes que o WebGPU oferece. Eles permitem executar cargas de trabalho computacionais arbitrárias a velocidades só possíveis graças à enorme quantidade de núcleos das GPUs modernas. Você pode executar modelos de aprendizado de máquina (machine learning) na web, realizar manipulação de imagens sem precisar configurar as etapas do pipeline de renderização (como processamento de vértices e fragment shading), processar números massivos de partículas, animar centenas de personagens com rig, etc.

Há uma grande variedade de tópicos que poderíamos cobrir, e o que você especificamente deseja fazer com compute shaders pode não estar listado aqui, mas esperamos que seja o suficiente para você começar. Além disso, estou testando um novo formato onde incluo menos código boilerplate e foco mais nos conceitos. O código completo continuará linkado no final do artigo se você ficar preso na sua implementação.

## Por que a computação na GPU é rápida

GPUs são geralmente consideradas mais rápidas que CPUs, mas isso tecnicamente não é exato. A velocidade de processamento de um núcleo de GPU é semelhante à da CPU, às vezes até mais lenta. De acordo com a [NVIDIA](https://www.nvidia.com/en-us/geforce/graphics-cards/compare/), a maioria das suas placas modernas tem frequências de clock em torno de 2.5 GHz. A [Qualcomm divulga](https://www.qualcomm.com/products/mobile/snapdragon/laptops-and-tablets/snapdragon-x-elite) que o Snapdragon X Elite tem frequências de clock de 3.4 a 4.3 GHz.

Então por que as GPUs são tão populares para grandes cargas de trabalho computacionais?

A resposta é a contagem de núcleos. O Snapdragon X Elite possui 12 núcleos. A RTX 5090 possui impressionantes 21760 núcleos. Isso representa uma diferença de 4 ordens de grandeza. Fazendo uma conta rápida de padeiro: se um algoritmo leva um segundo para executar uma operação na CPU e 2 segundos na GPU, para 12000 itens a CPU levará 1000 segundos (cerca de 16 minutos), enquanto a GPU levará 2 segundos (sem contabilizar o tempo de envio de dados para/da GPU e o tempo de setup).

Talvez uma demonstração ajude a ilustrar:

<iframe width="560" height="315" src="https://www.youtube.com/embed/vGWoV-8lteA?si=Sgl2Qq0CFoaGXMQa" title="YouTube video player" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>

GPUs são rápidas porque conseguem fazer milhares de coisas ao mesmo tempo. Dito isso, nem todos os algoritmos se beneficiam de aproveitar essa capacidade computacional.

## Quando devo usar compute pipelines?

É impossível fazer uma lista exaustiva de tudo para o qual você poderia usar uma GPU, mas aqui estão algumas regras gerais:

- Tarefas facilmente paralelizáveis. GPUs não gostam de alternar tarefas frequentemente; portanto, se a computação precisa usar dados de operações anteriores de forma sequencial, compute shaders provavelmente serão mais lentos do que uma abordagem baseada em CPU. Se cada operação puder ser executada sem conhecimento prévio de outras operações, você obterá um grande ganho na GPU.
- Os dados já estão na GPU. Se você está trabalhando com dados de texturas ou modelos, frequentemente é mais rápido processá-los com um compute shader do que copiar os dados para a CPU, modificá-los e enviá-los de volta para a GPU.
- Você tem um volume massivo de dados. Em determinado momento, o tamanho dos dados supera o tempo de setup e a complexidade de usar um compute pipeline. Ainda assim, você precisará adequar sua abordagem aos dados e ao processamento necessário.

Com isso esclarecido, vamos começar!

## Configurando o device e a queue

Usar compute shaders exige muito menos código do que usar um render pipeline. Não precisamos de uma janela, então podemos obter uma instância WGPU, solicitar um adapter e solicitar um device e uma queue com este código simples:

```rust
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = instance.request_adapter(&Default::default()).await.unwrap();
    let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();
```

<Note>

Estou usando [pollster](https://docs.rs/pollster) para tratar o `async` no código nativo nestes exemplos. Você pode usar a implementação `async` que preferir. Também uso [anyhow](https://docs.rs/anyhow) para tratamento de erros e [flume](https://docs.rs/flume) para a implementação de canais `async`.

</Note>

Se você quiser mais informações sobre essas chamadas e os possíveis argumentos que pode passar para elas, confira o [guia de renderização](../../beginner/tutorial2-surface/).

Agora que temos um device para nos comunicar com a GPU, vamos entender como configurar um compute pipeline.

## Compute Pipelines

Compute pipelines são muito mais simples de configurar do que render pipelines. Não precisamos configurar o pipeline de vértices tradicional. Veja só:

```rust
    let shader = device.create_shader_module(wgpu::include_wgsl!("introduction.wgsl"));

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Introduction Compute Pipeline"),
        layout: None,
        module: &shader,
        entry_point: None,
        compilation_options: Default::default(),
        cache: Default::default(),
    });
```

Estou usando os valores padrão para quase tudo aqui, exceto o `label` e o `module` do shader contendo o código do shader. Não estou especificando um bind group `layout`, o que significa que o wgpu derivará o layout a partir do código do shader. Não informo um `entry_point` porque o WGPU selecionará uma função com a tag `@compute` caso só exista uma no arquivo.

O código do shader para este exemplo também é simples:

```wgsl
// Um storage buffer somente leitura que armazena um array de inteiros de 32 bits sem sinal
@group(0) @binding(0) var<storage, read> input: array<u32>;
// Este storage buffer pode ser lido e escrito
@group(0) @binding(1) var<storage, read_write> output: array<u32>;

// Informa ao wgpu que esta função é um entry_point de compute pipeline válido
@compute
// Especifica a "dimensão" deste workgroup
@workgroup_size(64)
fn main(
    // global_invocation_id especifica nossa posição na grade de invocação
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>
) {
    let index = global_invocation_id.x;
    let total = arrayLength(&input);

    // O workgroup_size pode não ser um múltiplo do tamanho do array,
    // então precisamos encerrar a thread que indexaria fora dos limites.
    if (index >= total) {
        return;
    }

    // uma operação simples de cópia
    output[global_invocation_id.x] = input[global_invocation_id.x];
}
```

Este shader é muito simples. Tudo o que ele faz é copiar os conteúdos de um buffer para outro.
O único conceito que merece um pouco mais de explicação é o de workgroups e `workgroup_size`.

## Workgroups

Embora as GPUs prefiram que cada thread processe seu trabalho cegamente, problemas do mundo real exigem algum grau de sincronização. Compute shaders realizam isso por meio de workgroups.

Um workgroup é um grupo de `X * Y * Z` threads que compartilham informações sobre uma tarefa. Definimos o tamanho desse workgroup usando a anotação `workgroup_size`. Vimos uma versão simplificada acima, mas aqui está a versão completa:

```wgsl
@workgroup_size(64, 1, 1)
```

Isso significa que nosso compute shader criará workgroups com `64 * 1 * 1` threads, o que se reduz a 64 threads por workgroup. Se em vez disso usássemos:

```wgsl
@workgroup_size(64, 64, 1)
```

Teríamos `64 * 64 * 1` threads, ou seja, 4096 threads por workgroup.

O tamanho máximo suportado de workgroup pode variar dependendo do seu dispositivo, mas a especificação WebGPU garante o seguinte:

- Tamanho máximo de workgroup em X: 256
- Tamanho máximo de workgroup em Y: 256
- Tamanho máximo de workgroup em Z: 64
- Tamanho total do workgroup (X * Y * Z): 256

Isso significa que talvez não seja possível usar `@workgroup_size(64, 64, 1)`, mas `@workgroup_size(16, 16, 1)` funcionará na maioria dos dispositivos.

<Note>

### Por que XYZ?

Muitos dados em programação gráfica/GPU vêm organizados em arrays 2D ou até 3D. Por essa razão, `workgroup_size` utiliza 3 dimensões em vez de 1 para tornar a escrita de código multidimensional mais conveniente.

Por exemplo, um desfoque (blur) em uma imagem 2D se beneficia de um workgroup 2D para que cada thread corresponda a um pixel da imagem. Já uma implementação de marching cubes se beneficia de um workgroup 3D, onde cada thread lida com a geometria de um voxel na grade 3D.

</Note>

## O global invocation id

Cada thread em um workgroup tem um ID associado que identifica a qual workgroup ela pertence. Podemos acessar isso usando a variável builtin `workgroup_id`.

```wgsl
@compute
@workgroup_size(64)
fn main(
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
) {
    // ...
}
```

Saber onde estamos dentro do próprio workgroup também é útil, e fazemos isso usando o builtin `local_invocation_id`.

```wgsl
@compute
@workgroup_size(64)
fn main(
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_invocation_id: vec3<u32>,
) {
    // ...
}
```

Podemos então calcular nossa posição global na grade de invocação dos workgroups usando:

```wgsl
let id = workgroup_id * workgroup_size + local_invocation_id;
```

Ou podemos simplesmente usar a builtin `global_invocation_id`, como fizemos no código do shader mostrado anteriormente.

### De onde vem o workgroup_id?

Quando despachamos nosso compute shader, precisamos especificar as dimensões X, Y e Z do que é chamado de "grade do compute shader". Considere este código:

```rust

    {
        // Especificamos 64 threads por workgroup no shader, então precisamos calcular quantos
        // workgroups devemos despachar.
        let num_dispatches = input_data.len().div_ceil(64) as u32;

        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(num_dispatches, 1, 1);
    }
```

Na chamada `pass.dispatch_workgroups()`, usamos uma grade com dimensões `(num_dispatches, 1, 1)`, o que significa que lançaremos `num_dispatches * 1 * 1` workgroups. A GPU atribui a cada workgroup um ID com a coordenada x variando entre 0 e `num_dispatches - 1`.

É importante ter ciência disso porque, se você alterar o tamanho do workgroup, o `global_invocation_id` resultante mudará, significando que você pode potencialmente usar mais threads do que precisa ou menos do que o suficiente.

## Buffers

Embora já tenhamos abordado buffers no [guia de renderização](../../beginner/tutorial4-buffer/), farei uma breve revisão aqui. No WebGPU, um buffer é um trecho de memória reservado na GPU. Essa memória pode ser usada para qualquer coisa, desde dados de vértices até neurônios de uma rede neural. Em geral, a GPU não se importa com o significado dos dados contidos no buffer, mas se importa com como esses dados serão utilizados.

Aqui está um exemplo de configuração de buffer de entrada (input) e saída (output):

```rust
    let input_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("input"),
        contents: bytemuck::cast_slice(&input_data),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
    });

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("output"),
        size: input_buffer.size(),
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });
```

Precisamos especificamente do uso `STORAGE` para nosso buffer neste shader. Poderíamos usar `UNIFORM` para algumas coisas, mas buffers uniform são mais limitados quanto ao tamanho e não podem ser modificados dentro do shader.

## Configuração de Bindgroup

Novamente, não entrarei em detalhes minuciosos sobre como definir bind groups aqui, já que fizemos isso no [guia de renderização](../../beginner/tutorial5-textures/), mas revisaremos a teoria. No WebGPU, um bind group descreve recursos que podem ser consumidos pelo shader. Podem ser texturas, buffers, samplers, etc. Um `BindGroupLayout` define como esses recursos estão agrupados, quais estágios de shader têm acesso a eles e como o shader interpretará esses recursos.

Você pode especificar o `BindGroupLayout` manualmente, mas o WGPU consegue inferir o layout com base no código do shader. Por exemplo:

```wgsl
@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;
```

O WGPU interpreta isso como um layout com 2 entradas: um storage buffer somente leitura chamado `input` na binding 0, e um storage buffer de leitura e escrita chamado `output` na binding 1. Podemos facilmente criar um bind group que satisfaça isso com o seguinte código:

```rust
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output_buffer.as_entire_binding(),
            },
        ],
    });
```

## Obtendo dados da GPU

Dependendo das necessidades da sua aplicação, os dados processados em um compute shader podem permanecer na GPU se forem usados apenas para renderização ou outros compute pipelines. Se você precisar transferir esses dados da GPU para a CPU, ou se desejar apenas inspecioná-los, existe uma maneira de fazer isso.

O processo exige alguns passos, então vejamos o código:

```rust
    {
        // O processo de mapeamento é assíncrono, então precisaremos criar um canal para receber
        // a flag de sucesso do nosso mapeamento
        let (tx, rx) = channel();

        // Enviamos o sucesso ou falha do nosso mapeamento via callback
        temp_buffer.map_async(wgpu::MapMode::Read, .., move |result| tx.send(result).unwrap());

        // O callback enviado para map_async só será chamado após o
        // device ser consultado (poll) ou a queue ser enviada
        device.poll(wgpu::PollType::wait_indefinitely())?;

        // Verificamos aqui se o mapeamento foi bem-sucedido
        rx.recv()??;

        // Em seguida, obtemos os bytes que estavam armazenados no buffer
        let output_data = temp_buffer.get_mapped_range(..)?;

        // Agora que temos os dados na CPU, podemos fazer o que quisermos com eles
        assert_eq!(&input_data, bytemuck::cast_slice(&output_data));
    }

    // Precisamos desmapear o buffer para poder usá-lo novamente
    temp_buffer.unmap();
```

Você deve ter notado que usei uma variável chamada `temp_buffer` e não `output_buffer` no mapeamento. O motivo para isso é que o buffer mapeado precisa ter o uso `MAP_READ`. Esse uso só é compatível com o uso `COPY_DST`, o que significa que ele não pode ter os usos `STORAGE` nem `UNIFORM`, ou seja, não podemos usar esse buffer diretamente em um compute shader. Contornamos isso criando um buffer temporário para o qual copiamos o `output_buffer` e, em seguida, mapeamos o buffer temporário. Aqui está o código de configuração do `temp_buffer`:

```rust
    let temp_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("temp"),
        size: input_buffer.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
```

Precisamos realizar essa cópia antes de submeter a queue:

```rust
    encoder.copy_buffer_to_buffer(&output_buffer, 0, &temp_buffer, 0, output_buffer.size());

    queue.submit([encoder.finish()]);
```

## Conclusão

É isso! Nada muito difícil, especialmente se comparado à configuração de um render pipeline. Agora que sabemos como usar um compute pipeline, podemos começar a fazer coisas mais interessantes. Este guia não tem como cobrir todas as formas de utilizar compute shaders, mas pretendo abordar alguns dos blocos fundamentais necessários para construir a maioria dos algoritmos. A partir disso, você poderá pegar esses conceitos e aplicá-los nos seus próprios projetos!

## Demonstração

<WasmExample example="compute" noCanvas="true" autoLoad="true"></WasmExample>

<AutoGithubLink path="/compute/src/"/>
