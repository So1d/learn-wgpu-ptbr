# Ordenação (Sorting) na GPU

Lidar com dados ordenados facilita a elaboração da maioria dos algoritmos, portanto faz sentido querer ordenar nossos dados diretamente na GPU. Precisamos repensar a forma como abordamos a ordenação, pois os algoritmos tradicionais de ordenação não foram projetados tendo em mente o poder computacional paralelo. Felizmente, existem alguns algoritmos que funcionam muito bem na GPU!

## Odd-Even Sort (ou Brick Sort)

Esta ordenação (par-ímpar) funciona iterando sobre pares de itens, comparando-os e trocando-os de posição se um for maior que o outro. Considere o seguinte array:

```rust
[3, 7, 1, 5, 0, 4, 2, 6]
```

Primeiro fazemos o passo ímpar (odd pass). Isso significa que consideramos pares de itens a partir do índice 1 (não 0). Para os dados acima, os pares seriam:

```rust
[[7, 1], [5, 0], [4, 2]]
```

Pulamos o primeiro e o último elemento, pois eles não possuem outro número adjacente que não esteja já pareado. Então trocamos o número maior com o número menor, o que resulta no seguinte estado dos dados:

```rust
[3, 1, 7, 0, 5, 2, 4]
```

Em seguida, vem o passo par (even pass). É idêntico ao passo ímpar, mas começamos no índice 0 em vez de 1. Isso nos dá os seguintes pares:

```rust
[[3, 1], [7, 0], [2, 4]]
```

Que, ao trocar os elementos fora de ordem, resulta em:

```rust
[1, 3, 0, 7, 2, 4]
```

Esse processo se repete até que o array esteja completamente ordenado. Aqui estão os dados em cada iteração subsequente:

```rust
[1, 0, 3, 2, 7, 4] // ímpar (odd)
[0, 1, 2, 3, 4, 7] // par (even)
```

## Quando paramos a ordenação?

A maioria dos algoritmos de ordenação não verifica manualmente se o array está ordenado a cada iteração. Felizmente, [pesquisas mostram](https://en.wikipedia.org/wiki/Odd%E2%80%93even_sort#cite_note-6) que o número máximo de iterações para concluir este algoritmo é igual ao número de itens, isto é, N. Isso significa que, para um array de tamanho `N = 8`:

```rust
[7, 6, 5, 4, 3, 2, 1, 0]
```

Levará 8 passos para ordenar esses dados.

```rust
[7, 5, 6, 3, 4, 1, 2, 0] // ímpar (odd)
[5, 7, 3, 6, 1, 4, 0, 2] // par (even)
[5, 3, 7, 1, 6, 0, 4, 2] // ímpar (odd)
[3, 5, 1, 7, 0, 6, 2, 4] // par (even)
[3, 1, 5, 0, 7, 2, 6, 4] // ímpar (odd)
[1, 3, 0, 5, 2, 7, 4, 6] // par (even)
[1, 0, 3, 2, 5, 4, 7, 6] // ímpar (odd)
[0, 1, 2, 3, 4, 5, 6, 7] // par (even)
```

Isso sempre funcionará, independentemente dos dados que estamos ordenando. É um pouco ineficiente quando os dados já estão quase ordenados, por isso é bom manter esse ponto em mente.

## Portando Odd-Even Sort para WGSL

O Odd-Even sort é especial porque cada passo é trivial de paralelizar. Cada par de itens é considerado de forma independente de todos os outros itens. Isso significa que podemos dedicar uma única thread para cada par que desejamos comparar. Vamos direto ao shader!

```wgsl
@group(0)
@binding(0)
var<storage, read_write> data: array<u32>;

@compute
@workgroup_size(64, 1, 1)
fn odd_even_sort(
    @builtin(global_invocation_id)
    gid: vec3<u32>,
) {
    // ...
}
```

Isso funciona de maneira muito parecida com o código de introdução. Configuramos nosso bind group com um `array<u32>` do tipo `read_write`, já que esta ordenação funciona sem a necessidade de um array adicional para saída. Também precisamos apenas da builtin `global_invocation_id` para indexar adequadamente nossos `data`. Agora examinaremos o código dentro de `odd_even_sort`.

```wgsl
    let num_items = arrayLength(&data);
    let pair_index = gid.x;
```

Primeiro, obtemos o índice do par no qual a thread atual está trabalhando.

```wgsl
    // ímpar (odd)
    var a = pair_index * 2u + 1;
    var b = a + 1u;

    if a < num_items && b < num_items && data[a] > data[b] {
        let temp = data[a];
        data[a] = data[b];
        data[b] = temp;
    }
```

Para esta parte do código, primeiro obtemos os índices dos itens que queremos comparar. Se os índices estiverem dentro dos limites e os valores estiverem fora de ordem, trocamos os valores. Podemos incluir o passo par também para que possamos reduzir pela metade o número de vezes que chamamos este shader.

```wgsl
    // par (even)
    a = pair_index * 2u;
    b = a + 1u;

    if a < num_items && b < num_items && data[a] > data[b] {
        let temp = data[a];
        data[a] = data[b];
        data[b] = temp;
    }
```

Parece que terminamos o código do shader, mas tecnicamente existe um erro no nosso código. Não tem a ver com a lógica do nosso algoritmo, mas sim com a natureza da programação paralela em geral: condições de corrida (race conditions).

## Condições de corrida (Race conditions) e barreiras (barriers)

![exemplo de race conditions ilustrado por filhotes](./race-condition-puppies.jpg)

Uma condição de corrida (race condition) ocorre quando duas ou mais threads tentam operar sobre a mesma posição de memória. Se fizéssemos um passo para cada chamada ao shader, estaríamos seguros, mas como fazemos dois passos na mesma execução, precisamos garantir que as threads não interfiram umas nas outras. Fazemos isso usando barreiras (barriers).

Uma barreira faz com que a thread atual aguarde a conclusão das outras threads antes de prosseguir. Existem dois tipos de barreira:

Um `workgroupBarrier` fará com que todas as threads do workgroup aguardem até que todas as outras threads daquele mesmo workgroup tenham atingido a barreira. Ele também sincronizará todas as variáveis atômicas e dados armazenados no espaço de endereçamento (address space) do workgroup.

<Note>

Em WGSL, o "address space" determina como um determinado bloco de memória pode ser acessado. Dados no espaço de endereçamento `workgroup` só são acessíveis por threads dentro do mesmo workgroup. Muitos espaços de endereçamento são implícitos, como o `function`. Os espaços `uniform` e `storage` são significativos por corresponderem respectivamente a uniform buffers e storage buffers.

</Note>

Um `storageBarrier` fará com que a GPU sincronize todas as alterações feitas nos storage buffers. Como nossos dados estão em um storage buffer, esta é a barreira necessária para garantir que os dados permaneçam sincronizados. Adicione a seguinte linha entre os passos ímpar e par:

```wgsl
    storageBarrier();
```

Com isso, a função `odd_even_sort` fica assim:

```wgsl
@compute
@workgroup_size(64, 1, 1)
fn odd_even_sort(
    @builtin(global_invocation_id)
    gid: vec3<u32>,
) {
    let num_items = arrayLength(&data);
    let pair_index = gid.x;

    // ímpar (odd)
    var a = pair_index * 2u + 1;
    var b = a + 1u;

    if a < num_items && b < num_items && data[a] > data[b] {
        let temp = data[a];
        data[a] = data[b];
        data[b] = temp;
    }

    storageBarrier();

    // par (even)
    a = pair_index * 2u;
    b = a + 1u;

    if a < num_items && b < num_items && data[a] > data[b] {
        let temp = data[a];
        data[a] = data[b];
        data[b] = temp;
    }
}
```

## Chamando o shader

A maior parte do código Rust é igual à da introdução, com exceção de criar apenas um storage buffer e o seguinte trecho para chamar o shader:

```rust
    let num_items_per_workgroup = 128; // 64 threads, 2 itens por thread
    let num_dispatches = (input_data.len() / num_items_per_workgroup) as u32
        + (input_data.len() % num_items_per_workgroup > 0) as u32;
    // Realizamos 2 passos no shader, então só precisamos executar metade das iterações
    let num_passes = input_data.len().div_ceil(2);

    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);

        for _ in 0..num_passes {
            pass.dispatch_workgroups(num_dispatches, 1, 1);
        }
    }
```

Com isso, seus dados devem estar ordenados. Você pode agora usá-los para qualquer finalidade necessária, como ordenar objetos transparentes pela coordenada Z, ou ordenar objetos pela célula da grade a que pertencem para detecção e resolução de colisões. Estaremos usando ordenação para implementar diferentes algoritmos em outras partes deste guia.

## Conclusão

A ordenação é um dos pilares do desenvolvimento de software e agora podemos ordenar nossos dados na GPU sem precisar enviá-los em uma viagem de ida e volta para a CPU. Usaremos bastante isso no restante deste guia.

Obrigado pela leitura, e um agradecimento especial a estes apoiadores (patrons)!

* Filip
* Lions Heart
* Jani Turkia
* Julius Liu
* 折登 樹
* Aron Granberg
* Ian Gowen
* Bernard Llanos
* David Laban
* IC

<WasmExample example="compute" noCanvas="true" autoLoad="true"></WasmExample>

<AutoGithubLink path="/compute/src/"/>
