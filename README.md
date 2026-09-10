# schematize_optimizer_gui_rs

A janela (Slint) do `schematize-optimizer`.

## O que ela é

Uma casca **visual fina** por cima do binário headless. Duas telas — o diagnóstico da máquina e
os tetos por software —, lendo `diag --json`, `limits --json` e `services --json`. Nada mais.

## O que justifica ela existir

O ícone do optimizer executava `diag --wait`: abria um terminal, imprimia um relatório e
esperava um Enter. **Ícone que só imprime é relatório com atalho, não aplicativo.**

Mas espelhar a CLI numa janela seria uma CLI pior, com mouse. O que justifica a janela é uma
coisa só, e está na tela de limites: **mostrar o que MUDA na máquina antes de mudar.** No
terminal, `limits` diz o que sugeriria e `limits --apply` escreve; entre os dois não há nada
que compare o sugerido com o que já está no disco. Aqui há.

## O que ela NÃO faz, e a ausência é deliberada

- **Não aplica nada de dentro.** Mexer em systemd pede sudo, e a senha precisa de onde ser
  digitada — aplicar e reverter abrem um TERMINAL. Progresso, `Ctrl-C` e erro copiável vêm de
  graça, e são de verdade em vez de desenhados.
- **Não tem botão para o parâmetro de kernel.** Mudar o `amdgpu.gttsize` exige editar o cmdline
  do boot e reiniciar. Um botão ali prometeria um clique onde há um reboot.
- **Não depende do crate `optimizer`.** Depender dele a faria embutir a versão via git-dep —
  exatamente o que fez a janela irmã "abrir a versão antiga".

## A fronteira entre casca e domínio

`cli.rs` e `terminal.rs` são **plataforma**: não sabem o que é limite, slice, GTT ou serviço.
Há um teste que lê o próprio `cli.rs` e reprova se uma palavra do negócio aparecer no código —
a regra do D4 virando verificação em vez de recomendação.

`telas.rs` é o domínio, e é **puro**: entra texto, sai modelo. É o que permite testar as duas
telas inteiras sem o optimizer instalado.

## Rodar

```sh
cargo run --release              # abre no Diagnóstico
cargo run --release -- --limites # abre em Limites
```
