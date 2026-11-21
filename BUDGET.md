# BUDGET — orçamento de memória (RSS de pico)

Piso de eficiência de runtime da casa (`eficiencia-runtime.md`). Teto da casa para GUI: 400 MiB.

| Operação | RSS pico alvo (release) | Medido release (antes -> depois) | Medido debug (teste) | Teto no teste | Como medir |
|---|---|---|---|---|---|
| Abrir a janela e desenhar a 1a tela (5 s) | <= 24 MiB | 16,8 -> 16,8 MiB (binário instalado, `VmHWM`) | 30,0 MiB | 40 MiB | `cargo test --test orcamento -- --nocapture` (precisa de `DISPLAY` ou `WAYLAND_DISPLAY`; sem display o teste AVISA e pula) |

Teto = medido (debug) + ~30%. O teste polla `VmHWM` de `/proc/<pid>/status`.

## Diagnóstico

Nenhuma mudança de código: a casca só orquestra o binário `schematize-optimizer` e captura o `--json` (limitado pelo próprio CLI). `slint` já está com `default-features = false` (winit + femtovg, sem skia), ~527 pacotes (`cargo tree --prefix none | sort -u`), ~36 crates duplicados em versões (todos do stack winit/wayland/slint, fora do alcance daqui), binário release instalado de 18,7 MiB. 16,8 MiB está muito abaixo dos 400 MiB; não há ganho medido a buscar.

Não medido: telas pesadas (`disco` com muitos achados) na janela, que dependem de interação humana.
