//! A janela (Slint) do `schematize-optimizer`.
//!
//! **O quê:** casca VISUAL fina por cima do binário headless. Duas telas — o diagnóstico da
//! máquina, e os tetos por software — lendo o `--json`, e nada mais.
//!
//! **Onde:** o ícone do optimizer, e `schematize-optimizer-gui` no terminal.
//!
//! ## Por que esta janela existe, e o que ela faz que a CLI não faz
//!
//! O ícone do optimizer executava `diag --wait`: abria um terminal, imprimia um relatório e
//! esperava um Enter. **Ícone que só imprime é relatório com atalho, não aplicativo.**
//!
//! Mas espelhar a CLI numa janela seria uma CLI pior, com mouse. O que justifica a janela é
//! uma coisa só, e está na tela de limites: **mostrar o que MUDA na máquina antes de mudar.**
//! No terminal, `limits` diz o que sugeriria e `limits --apply` escreve; entre os dois não há
//! nada que compare o sugerido com o que já está no disco. Aqui há, e é a razão da tela.
//!
//! ## O que ela NÃO faz, e a ausência é deliberada
//!
//! **Não aplica nada de dentro.** Mexer em systemd pede sudo, e a senha precisa de onde ser
//! digitada — então aplicar e reverter abrem um TERMINAL (D6). Progresso, `Ctrl-C` e erro
//! copiável vêm de graça, e são de verdade em vez de desenhados.
//!
//! **Não tem botão para o parâmetro de kernel.** Mudar o `amdgpu.gttsize` exige editar o
//! cmdline do boot e reiniciar. Um botão ali prometeria um clique onde há um reboot.
//!
//! **Não depende do crate `optimizer`.** Depender dele a faria embutir a versão via git-dep —
//! exatamente o que fez a janela irmã "abrir a versão antiga".
#![windows_subsystem = "windows"]

mod cli;
mod json;
mod telas;
mod terminal;

slint::include_modules!();

use std::cell::RefCell;
use std::rc::Rc;

/// **O quê:** aplica o diagnóstico na janela — inclusive quando ele é `Err`.
///
/// **Onde:** a abertura e o botão de recarregar.
///
/// **O `Err` NÃO vira fichas vazias.** Fichas em branco seriam a janela afirmando "esta
/// máquina não tem nada", quando o que houve foi não ter conseguido perguntar. É a confusão
/// entre "não sei" e "não tem" que quebrou a janela irmã, e ali ela custou uma versão inteira.
fn aplicar_diag(w: &MainWindow, r: Result<telas::Diag, String>) {
    match r {
        Ok(d) => {
            w.set_erro(slint::SharedString::new());
            w.set_versao(d.versao.into());
            w.set_ram(d.ram.into());
            w.set_nucleos(d.nucleos.into());
            w.set_cgroups(d.cgroups.into());
            w.set_cgroups_ok(d.cgroups_ok);
            w.set_gpu(d.gpu.into());
            w.set_vram(d.vram.into());
            w.set_gtt(d.gtt.into());
            w.set_limitavel(d.limitavel);
            w.set_parametro(d.parametro.into());
            w.set_atual(d.atual.into());
            w.set_sugerido(d.sugerido.into());
            w.set_motivo(d.motivo.into());
        }
        Err(e) => w.set_erro(e.into()),
    }
}

/// **O quê:** aplica os limites na janela — inclusive quando são `Err`.
///
/// **Onde:** a abertura, o recarregar, e depois de aplicar/reverter.
///
/// **O erro dos limites é SEPARADO do erro do diagnóstico** (`limites-erro`, não `erro`): a
/// tela de diagnóstico pode ter carregado bem e a de limites não — `limits --json` falha
/// sozinho quando falta cgroups v2, por exemplo. Um erro só, compartilhado, apagaria a tela
/// que estava funcionando.
fn aplicar_limites(w: &MainWindow, r: Result<telas::Limites, String>) {
    match r {
        Ok(l) => {
            w.set_limites_erro(slint::SharedString::new());
            w.set_diferenca(l.diferenca.into());
            w.set_ha_mudanca(l.ha_mudanca);
            let modelo: Vec<CaixaUI> = l
                .caixas
                .iter()
                .map(|c| CaixaUI {
                    nome: c.nome.clone().into(),
                    descricao: c.descricao.clone().into(),
                    memoria_apertar: c.memoria_apertar.clone().into(),
                    memoria_matar: c.memoria_matar.clone().into(),
                    cpu: c.cpu.clone().into(),
                    io: c.io.clone().into(),
                    aplicada: c.aplicada,
                })
                .collect();
            w.set_caixas(slint::ModelRc::new(slint::VecModel::from(modelo)));
        }
        Err(e) => {
            w.set_limites_erro(e.into());
            // Sem dado, não há o que aplicar — e o botão desabilitado é o que impede um clique
            // que rodaria um comando sobre um estado que a janela não conseguiu ler.
            w.set_ha_mudanca(false);
            w.set_diferenca("não consegui ler os tetos desta máquina.".into());
        }
    }
}

/// **O quê:** aplica os serviços de boot na janela.
///
/// **Onde:** a abertura e o recarregar.
///
/// **Falha aqui é SILENCIOSA de propósito** — a lista é secundária, e um erro dela ocupando a
/// tela esconderia o que é primário. Sem systemd, a lista fica vazia e o cabeçalho mostra
/// zero, que é a verdade.
fn aplicar_servicos(w: &MainWindow, r: Result<Vec<telas::Servico>, String>) {
    let modelo: Vec<ServicoUI> = r
        .unwrap_or_default()
        .iter()
        .map(|s| ServicoUI {
            unidade: s.unidade.clone().into(),
            custo: s.custo.clone().into(),
            auditado: s.auditado,
            recomendado: s.recomendado,
            explicacao: s.explicacao.clone().into(),
        })
        .collect();
    w.set_servicos(slint::ModelRc::new(slint::VecModel::from(modelo)));
}

/// **O quê:** lê tudo e aplica na janela. **Onde:** a abertura e o botão de recarregar.
fn recarregar(w: &MainWindow) {
    aplicar_diag(w, cli::ler_json(&["diag"]).and_then(|t| telas::ler_diag(&t)));
    aplicar_limites(w, cli::ler_json(&["limits"]).and_then(|t| telas::ler_limites(&t)));
    aplicar_servicos(w, cli::ler_json(&["services"]).and_then(|t| telas::ler_servicos(&t)));
}

/// **O quê:** dispara `limits --apply` ou `limits --revert` num TERMINAL (D6).
///
/// **Onde:** os dois botões da tela de limites.
///
/// **Sem terminal a janela NÃO some com o problema:** ela põe o comando na tela, para a pessoa
/// rodar onde quiser. Um botão que não faz nada e não diz nada é o que o §37.48 chama de bug
/// do software, não erro de quem clicou.
fn agir(flag: &str) -> String {
    let bin = cli::bin().display().to_string();
    let cmd = cli::comando_para_terminal(&bin, &["limits", flag]);
    if terminal::abrir(&cmd) {
        "terminal aberto — a senha do sudo é pedida lá. Depois, clique em «Recarregar».".to_string()
    } else {
        format!("não achei um terminal. Rode: {bin} limits {flag}")
    }
}

/// **O quê:** a aba em que a janela abre, lida da linha de comando.
///
/// **Onde:** [`main`]. `--limites` abre em Limites; sem argumento, em Diagnóstico.
///
/// **Por que existe:** o Diagnóstico responde "como está a máquina", que é a pergunta de quem
/// clicou no ícone sem pedir nada em especial. Mas quem chega pelo terminal já sabendo que
/// quer mexer nos tetos não deveria ter de passar por ela — e é a mesma flag que a janela
/// irmã tem, pela mesma razão.
///
/// **Argumento desconhecido não é erro.** Sair com erro por causa de uma flag digitada errado
/// trocaria uma janela que funciona por nenhuma.
fn aba_inicial(args: impl Iterator<Item = String>) -> i32 {
    for a in args {
        if a == "--limites" || a == "--limits" {
            return 1;
        }
    }
    0
}

fn main() -> Result<(), slint::PlatformError> {
    let w = MainWindow::new()?;
    w.set_aba(aba_inicial(std::env::args().skip(1)));
    recarregar(&w);

    {
        let weak = w.as_weak();
        w.on_recarregar(move || {
            if let Some(w) = weak.upgrade() {
                recarregar(&w);
                w.set_msg("estado relido.".into());
            }
        });
    }
    {
        let weak = w.as_weak();
        w.on_aplicar(move || {
            if let Some(w) = weak.upgrade() {
                let m = agir("--apply");
                w.set_msg(m.into());
            }
        });
    }
    {
        let weak = w.as_weak();
        w.on_reverter(move || {
            if let Some(w) = weak.upgrade() {
                let m = agir("--revert");
                w.set_msg(m.into());
            }
        });
    }
    {
        let weak = w.as_weak();
        w.on_alternar_tema(move || {
            if let Some(w) = weak.upgrade() {
                w.set_dark(!w.get_dark());
            }
        });
    }
    {
        let weak = w.as_weak();
        // O estado do colapso vive aqui e não no `.slint` porque é o Rust que o alterna; um
        // `in-out` que os dois lados escrevem é onde nasce a divergência entre o que a janela
        // acha que está aberto e o que está.
        let abertos = Rc::new(RefCell::new(false));
        w.on_alternar_servicos(move || {
            if let Some(w) = weak.upgrade() {
                let novo = !*abertos.borrow();
                *abertos.borrow_mut() = novo;
                w.set_servicos_abertos(novo);
            }
        });
    }

    w.run()
}

#[cfg(test)]
mod tests_aba {
    use super::aba_inicial;

    /// Sem argumento, abre no Diagnóstico — a pergunta de quem clicou no ícone.
    #[test]
    fn sem_argumento_abre_no_diagnostico() {
        assert_eq!(aba_inicial(std::iter::empty()), 0);
    }

    /// `--limites` abre em Limites, nas duas grafias.
    #[test]
    fn limites_abre_em_limites() {
        for flag in ["--limites", "--limits"] {
            assert_eq!(aba_inicial([flag.to_string()].into_iter()), 1, "{flag}");
        }
        assert_eq!(aba_inicial(["-x".to_string(), "--limites".to_string()].into_iter()), 1);
    }

    /// **Argumento desconhecido NÃO derruba a janela.** Sair com erro por uma flag digitada
    /// errado trocaria uma janela que funciona por nenhuma.
    #[test]
    fn argumento_desconhecido_nao_derruba_nada() {
        assert_eq!(aba_inicial(["--nao-existe".to_string()].into_iter()), 0);
        assert_eq!(aba_inicial(["".to_string(), "-".to_string()].into_iter()), 0);
    }
}

#[cfg(test)]
mod tests {
    /// **A janela não aplica nada de dentro — ela abre TERMINAL (D6).**
    ///
    /// Este teste lê o próprio arquivo e exige que o comando de mudança passe pelo terminal.
    /// É a regra do D6 virando verificação: mexer em systemd pede sudo, e um `Command::new`
    /// direto daqui rodaria sem ter onde pedir a senha — falharia em silêncio, ou pior,
    /// funcionaria só para quem já tem sudo sem senha, que é a minoria.
    #[test]
    fn o_que_muda_a_maquina_passa_pelo_terminal() {
        let fonte = include_str!("main.rs");
        let producao = fonte.split("#[cfg(test)]").next().unwrap();
        assert!(producao.contains("terminal::abrir"), "as ações têm de abrir terminal");
        assert!(
            !producao.contains("Command::new"),
            "esta janela não roda processo direto: o que muda a máquina vai para o terminal (D6)"
        );
    }
}
