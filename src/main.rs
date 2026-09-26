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
mod procedencia;
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

/// **O quê:** escreve a aba Disco. **Onde:** [`recarregar`].
///
/// **O erro é PRÓPRIO da aba**, e não o erro global: o `disco` varre o sistema de arquivos e
/// pode falhar por permissão num diretório enquanto o `diag` e o `limits` respondem bem. Um
/// erro global apagaria as quatro telas por causa de uma.
fn aplicar_disco(w: &MainWindow, r: Result<telas::Disco, String>) {
    match r {
        Ok(d) => {
            w.set_disco_erro(slint::SharedString::new());
            w.set_disco_total(telas::tamanho(d.total_bytes).into());
            let totais = |v: &[telas::TotalDisco]| {
                slint::ModelRc::new(slint::VecModel::from(
                    v.iter()
                        .map(|x| TotalDiscoUI {
                            chave: x.chave.clone().into(),
                            tamanho: telas::tamanho(x.bytes).into(),
                            custa_rede: x.custa_rede,
                        })
                        .collect::<Vec<_>>(),
                ))
            };
            w.set_disco_por_montagem(totais(&d.por_montagem));
            w.set_disco_por_tipo(totais(&d.por_tipo));
            w.set_disco_achados(slint::ModelRc::new(slint::VecModel::from(
                d.achados
                    .iter()
                    .map(|a| AchadoUI {
                        caminho: a.caminho.clone().into(),
                        tipo: a.tipo.clone().into(),
                        tamanho: telas::tamanho(a.bytes).into(),
                        dias_parado: a.dias_parado as i32,
                        refaz: a.refaz.clone().into(),
                    })
                    .collect::<Vec<_>>(),
            )));
        }
        Err(e) => {
            // Listas VAZIAS com o erro na tela, e nunca vazias caladas: "nada recriável" sobre
            // uma falha de leitura faria a pessoa concluir que a máquina está limpa.
            w.set_disco_achados(slint::ModelRc::new(slint::VecModel::from(Vec::<AchadoUI>::new())));
            w.set_disco_total("—".into());
            w.set_disco_erro(e.into());
        }
    }
}

/// **O quê:** escreve a aba Agentes. **Onde:** [`recarregar`].
fn aplicar_agentes(w: &MainWindow, r: Result<telas::Agentes, String>) {
    match r {
        Ok(a) => {
            w.set_agentes_erro(slint::SharedString::new());
            // A frase de cima diz o NÚMERO e o estado, não só o número: "cabem 4" com 5
            // rodando seria uma tela que ignora o que está acontecendo nela mesma.
            w.set_ag_cabem(
                if a.available == 0 && a.running >= a.total_cap {
                    format!("Já no teto: {} em paralelo", a.total_cap)
                } else {
                    format!("Cabem mais {} agente(s) agora (teto {})", a.available, a.total_cap)
                }
                .into(),
            );
            w.set_ag_rodando(format!("{} rodando neste momento", a.running).into());
            w.set_ag_gargalo(a.gargalo().into());
            w.set_ag_threads(format!("{}", a.threads).into());
            w.set_ag_ram(format!("{} MB", a.mem_available_mb).into());
            w.set_ag_carga(format!("{:.2}", a.load1).into());
            w.set_ag_ram_apertada(a.ram_tight);
        }
        Err(e) => {
            w.set_ag_cabem("—".into());
            w.set_ag_gargalo("—".into());
            w.set_agentes_erro(e.into());
        }
    }
}

/// **O quê:** lê tudo e aplica na janela. **Onde:** a abertura e o botão de recarregar.
fn recarregar(w: &MainWindow) {
    aplicar_diag(w, cli::ler_json(&["diag"]).and_then(|t| telas::ler_diag(&t)));
    aplicar_limites(w, cli::ler_json(&["limits"]).and_then(|t| telas::ler_limites(&t)));
    aplicar_servicos(w, cli::ler_json(&["services"]).and_then(|t| telas::ler_servicos(&t)));
    aplicar_disco(w, cli::ler_json(&["disco"]).and_then(|t| telas::ler_disco(&t)));
    aplicar_agentes(w, cli::ler_json(&["agentes"]).and_then(|t| telas::ler_agentes(&t)));
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
        match a.as_str() {
            "--limites" | "--limits" => return 1,
            "--servicos" | "--services" => return 2,
            "--disco" | "--disk" => return 3,
            "--agentes" | "--agents" => return 4,
            _ => {}
        }
    }
    0
}

fn main() -> Result<(), slint::PlatformError> {
    // **RESPONDE `--version` E SAI, antes de qualquer coisa gráfica.**
    //
    // Sem isto a janela IGNORA a flag e ABRE — e quem perguntou fica esperando. Não é hipótese:
    // o `debugreport` do CLI pergunta a versão de cada binário do ecossistema, e a janela do hub
    // já teve exatamente este defeito. O `cmd_out` cortava no timeout e devolvia a primeira
    // linha do log de ambiente (`(WAYLAND_DISPLAY=wayland-0)`) como se fosse o número da versão.
    //
    // Responder e sair é o contrato mínimo de um binário de linha de comando — inclusive de um
    // que normalmente abre janela.
    if std::env::args().skip(1).any(|a| a == "--version" || a == "-V") {
        println!("schematize-optimizer-gui {}", procedencia::rotulo_versao());
        return Ok(());
    }

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
        w.on_disco_limpar(move || {
            let Some(w) = weak.upgrade() else { return };
            // **Apagar passa por TERMINAL (D6), e sem `--yes`.** O `disco-clean` mostra a
            // lista ANTES e pergunta; a poda de volumes do Docker pergunta mesmo com `--yes`,
            // porque volume é DADO. Um botão que apagasse daqui tiraria da pessoa a única
            // chance de ver o que vai embora — e o que vai embora é um caminho no disco dela.
            let bin = cli::bin().display().to_string();
            let cmd = cli::comando_para_terminal(&bin, &["disco-clean"]);
            let msg = if terminal::abrir(&cmd) {
                "terminal aberto — a lista aparece lá, e ele pergunta antes de apagar. Depois, clique em «Recarregar».".to_string()
            } else {
                format!("não achei um terminal. Rode: {cmd}")
            };
            w.set_msg(msg.into());
        });
    }
    {
        let weak = w.as_weak();
        // O estado do colapso vive aqui e não no `.slint` porque é o Rust que o alterna; um
        // `in-out` que os dois lados escrevem é onde nasce a divergência entre o que a janela
        // acha que está aberto e o que está.
        // Começa ABERTA: numa aba própria, a lista é a resposta. O valor inicial aqui tem de
        // bater com o do `.slint` — dois defaults diferentes fazem o primeiro clique parecer
        // que não funcionou.
        let abertos = Rc::new(RefCell::new(true));
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
mod tests_aba_extradicao {
    use super::aba_inicial;

    fn v(a: &[&str]) -> std::vec::IntoIter<String> {
        a.iter().map(|s| s.to_string()).collect::<Vec<_>>().into_iter()
    }

    /// **As CINCO abas abrem por flag, e em português E inglês.**
    ///
    /// É por aqui que o hub delega — ele já sabe em qual assunto a pessoa estava, e obrigá-la
    /// a achar a aba de novo seria a delegação perdendo a única informação que tinha.
    #[test]
    fn as_cinco_abas_abrem_por_flag() {
        for (flag, n) in [
            ("--limites", 1),
            ("--limits", 1),
            ("--servicos", 2),
            ("--services", 2),
            ("--disco", 3),
            ("--disk", 3),
            ("--agentes", 4),
            ("--agents", 4),
        ] {
            assert_eq!(aba_inicial(v(&[flag])), n, "{flag}");
        }
        assert_eq!(aba_inicial(v(&[])), 0, "sem flag, o Diagnóstico");
    }

    /// **Todo destino tem uma aba no `.slint`.** Sem isto, uma flag apontando para um número
    /// sem tela abriria a janela em BRANCO — e nada reprovaria, porque o Rust compila e o
    /// Slint não sabe que o número veio de uma flag.
    #[test]
    fn todo_destino_existe_no_slint() {
        let ui = include_str!("../ui/optimizer.slint");
        for n in 0..=4 {
            assert!(
                ui.contains(&format!("root.aba == {n}")),
                "a aba {n} não existe no optimizer.slint — a janela abriria em BRANCO"
            );
        }
        // Self-check: a 9 não existe, e se esta asserção parar de valer o varredor cegou.
        assert!(!ui.contains("root.aba == 9"), "o self-check parou de valer");
    }

    /// **Apagar disco passa por TERMINAL (D6), e sem `--yes`.**
    ///
    /// O `disco-clean` mostra a lista ANTES e pergunta; a poda de volumes do Docker pergunta
    /// mesmo com `--yes`, porque volume é DADO — banco de dev, upload de teste. Um botão que
    /// apagasse daqui tiraria da pessoa a única chance de ver o que vai embora.
    #[test]
    fn apagar_disco_nao_acontece_na_janela() {
        let fonte = include_str!("main.rs");
        let producao = fonte.split("#[cfg(test)]").next().expect("há código antes dos testes");
        let i = producao.find("on_disco_limpar").expect("o callback sumiu");
        let corpo = &producao[i..producao.len().min(i + 900)];
        assert!(corpo.contains("terminal::abrir"), "apagar tem de abrir TERMINAL");
        assert!(corpo.contains("\"disco-clean\""), "perdeu o subcomando");
        // **O varredor ignora COMENTÁRIO, e isso foi aprendido errando — duas vezes.**
        //
        // A primeira versão procurava `--yes` no trecho inteiro e reprovou por causa do
        // comentário MEU que diz *"pergunta mesmo com `--yes`"*. Proibir a palavra proíbe
        // explicar a regra; é o mesmo erro que a janela do database cometeu com
        // `Command::new` e a do deployer com `passphrase`. Comentário CITA; a linha USA.
        let linhas: Vec<&str> =
            corpo.lines().map(str::trim_start).filter(|l| !l.starts_with("//")).collect();
        assert!(
            !linhas.iter().any(|l| l.contains("--yes")),
            "`--yes` aqui tira a pergunta que protege o DADO"
        );
        // Self-check: o comentário que EXPLICA a regra tem de continuar passando.
        assert!(corpo.contains("`--yes`"), "proibir a palavra proibiria explicar a política");
        // E a janela não apaga arquivo por conta própria, em lugar nenhum.
        let codigo: Vec<&str> =
            producao.lines().map(str::trim_start).filter(|l| !l.starts_with("//")).collect();
        for proibido in ["remove_dir_all", "remove_file", "fs::remove"] {
            assert!(
                !codigo.iter().any(|l| l.contains(proibido)),
                "a janela apagou do disco (`{proibido}`) — quem apaga é o binário, no terminal"
            );
        }
        // Self-check: o varredor acha o que procura quando ele está lá.
        assert!(codigo.iter().any(|l| l.contains("terminal::abrir")), "o varredor está cego");
    }
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

#[cfg(test)]
mod tests_versao {
    /// **A janela responde a versão COM a procedência, como o CLI irmão.**
    ///
    /// Medido lado a lado: o CLI dizia `0.4.0 (b9fcc65)` e esta janela dizia `0.4.0` seco. O
    /// número sozinho não distingue dois binários com o mesmo `Cargo.toml` e comportamento
    /// diferente — e aqui isso não é hipótese: um binário de 15 dias atrás passou por novo e
    /// gravou o `.desktop` errado, porque a versão batia.
    #[test]
    fn a_versao_carrega_a_procedencia() {
        let fonte = include_str!("main.rs");
        let producao = fonte.split("#[cfg(test)]").next().expect("há código antes dos testes");
        assert!(
            !producao.contains("env!(\"CARGO_PKG_VERSION\")"),
            "a janela usou o número seco — use `procedencia::rotulo_versao()`, que traz o SHA"
        );
        assert!(producao.contains("procedencia::rotulo_versao()"), "a versão perdeu a procedência");
    }

    /// **A janela responde `--version` e SAI, em vez de abrir.**
    ///
    /// Lê o próprio fonte e exige que a checagem seja a PRIMEIRA coisa do `main` — antes de
    /// qualquer chamada gráfica. A ordem é o ponto: a flag tratada depois de abrir a janela
    /// responde tarde demais, e quem perguntou já está esperando.
    ///
    /// **O defeito é conhecido e já custou um relatório errado.** O `debugreport` do CLI
    /// pergunta a versão de cada binário do ecossistema; a janela do hub ignorava a flag e
    /// abria, o `cmd_out` cortava no timeout, e a primeira linha do log de ambiente
    /// (`(WAYLAND_DISPLAY=wayland-0)`) entrava no relatório como se fosse o número da versão.
    #[test]
    fn responde_version_antes_de_abrir_a_janela() {
        let fonte = include_str!("main.rs");
        let producao = fonte.split("#[cfg(test)]").next().unwrap();
        let i = producao.find("fn main(").expect("há um main");
        let corpo = &producao[i..];

        let versao = corpo.find("\"--version\"").expect(
            "a janela tem de responder `--version` — sem isso ela ABRE quando alguém pergunta",
        );
        // A checagem vem antes de tudo que toca a tela. `MainWindow::new()` é a primeira
        // chamada gráfica de todas.
        let janela = corpo.find("MainWindow::new").expect("o main cria a janela");
        assert!(
            versao < janela,
            "`--version` é tratado DEPOIS de criar a janela — responder tarde é o mesmo que \
             não responder, porque a janela já subiu"
        );
        assert!(corpo[versao..janela].contains("return"), "tem de RESPONDER e SAIR");
    }
}
