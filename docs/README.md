# Introdução

<Note :hidden="false">

**Fork de Tradução para Português Brasileiro (PT-BR)**

Este repositório é um *fork* de estudo pessoal do projeto oficial [learn-wgpu](https://github.com/sotrh/learn-wgpu) criado por [@sotrh](https://github.com/sotrh). Todo o conteúdo foi traduzido para Português Brasileiro (PT-BR) utilizando Inteligência Artificial (IA).

</Note>

## O que é o wgpu?

O [Wgpu](https://github.com/gfx-rs/wgpu) é uma implementação em Rust da [especificação da API WebGPU](https://gpuweb.github.io/gpuweb/). A WebGPU é uma especificação publicada pelo grupo *GPU for the Web Community Group*. O seu objetivo é permitir que o código web acesse funções da GPU de maneira segura e confiável. Ela faz isso imitando a API Vulkan e traduzindo essas chamadas para a API nativa que o hardware hospedeiro utiliza (como DirectX, Metal ou Vulkan).

O Wgpu continua em desenvolvimento ativo, portanto partes desta documentação podem sofrer alterações.

## Por que Rust?

Na verdade, o Wgpu possui bindings em C que permitem escrever código em C/C++, além de outras linguagens que possuem interface com C. Dito isso, o Wgpu foi escrito nativamente em Rust e oferece bindings convenientes em Rust sem a necessidade de contornar complexidades. Além disso, a experiência de desenvolvimento em Rust é excelente.

Você deve ter uma boa familiaridade com o Rust antes de seguir este tutorial, pois não entraremos em detalhes sobre a sintaxe básica da linguagem. Caso queira revisar o Rust, você pode consultar o [tutorial oficial de Rust](https://www.rust-lang.org/learn). Também é recomendável estar familiarizado com o [Cargo](https://doc.rust-lang.org/cargo/).

Este projeto foi construído no processo de aprendizado do próprio Wgpu, então sugestões e feedbacks construtivos são sempre bem-vindos.

## Contribuição e Suporte

* Pull requests são bem-vindos no ([repositório do GitHub](https://github.com/sotrh/learn-wgpu)) para correção de problemas neste tutorial, como erros de digitação, informações incorretas ou inconsistências.
* Devido à evolução constante da API do wgpu, pull requests com novas demonstrações de showcase não estão sendo aceitos no momento.
* Se quiser apoiar o autor original diretamente, confira o [patreon do sotrh](https://www.patreon.com/sotrh)!

## Traduções

* [Versão em Chinês (中文版): 增加了与 App 的集成与调试系列章节](https://jinleili.github.io/learn-wgpu-zh/)
* **Versão em Português Brasileiro (PT-BR)**: Esta documentação traduzida e mantida em PT-BR.

## Agradecimentos especiais aos apoiadores do Patreon

* David Laban
* Bernard Llanos
* Ian Gowen
* Aron Granberg
* 折登 樹
* Julius Liu
* Jani Turkia
* Lions Heart
* Filip
* IC
* papyDoctor
* Feng Liang
* Jan Šipr
* Joris Willems
* Mattia Samiolo
* Lennart
* Paul E Hansen
* Gunstein Vatnar
* Nico Arbogast
* Dude
* Youngsuk Kim
* Alexander Kabirov
* charlesk
* Danny McGee
* yutani
* Eliot Bolduc
* Ben Anderson
* Thunk
* Craft Links
* Zeh Fernando
* Ken K
* Ryan
* Felix
* Tema
* 大典 加藤
* Andrea Postal
* Davide Prati
* dadofboi
* Beryesa
* Dzianis Sheka
* George Offley
* Imbris
* Maximilian Temeschinko
* Michael Trainor
