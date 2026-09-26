//! O DOMÍNIO das duas telas — o `--json` do optimizer virando o que a janela desenha.
//!
//! **O quê:** lê `diag --json`, `limits --json` e `services --json`, e produz o modelo das
//! telas de diagnóstico e de limites.
//!
//! **Onde:** a janela. Tudo aqui é PURO — entra texto, sai modelo —, e é isso que permite
//! testar as duas telas inteiras sem o optimizer instalado na máquina de quem roda a suíte.
//!
//! ## Onde esta janela ganha da CLI, e por que é aqui
//!
//! No terminal, `limits` imprime o que sugeriria e `limits --apply` escreve. Entre os dois não
//! há nada: quem quer saber **o que muda** tem de ler a sugestão, lembrar o que já estava no
//! disco, e comparar de cabeça.
//!
//! O `limits --json` traz os dois lados — `boxes` (o que se sugere) e `applied` (o que já
//! está no disco) —, e é [`diferenca`] que os transforma na frase que a CLI não dá:
//! **isto aqui vai mudar na sua máquina**. É o único lugar em que uma janela é honestamente
//! melhor que uma linha de comando, e foi por isso que a tela existe.
//!
//! ## Nada aqui decide a partir de prosa
//!
//! O contrato do optimizer não tem prosa nenhuma: motivo de GPU e descrição de caixa viajam
//! como CHAVE de catálogo (`gpu.intel_firmware`, `box.build`), nunca como frase. Quem traduz
//! é esta janela, com uma tabela pequena de conjunto fechado — e é assim que o documento
//! continua byte a byte idêntico em qualquer idioma.

use crate::json::{self, Json};

// ---------------------------------------------------------------------------
// Tela 1 — DIAGNÓSTICO
// ---------------------------------------------------------------------------

/// O que a tela de diagnóstico mostra.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Diag {
    pub versao: String,
    pub ram: String,
    pub nucleos: String,
    pub cgroups: String,
    pub cgroups_ok: bool,
    pub gpu: String,
    pub vram: String,
    pub gtt: String,
    /// Dá para limitar a memória compartilhada nesta máquina?
    pub limitavel: bool,
    /// O parâmetro de kernel, quando dá.
    pub parametro: String,
    pub atual: String,
    pub sugerido: String,
    /// Por que NÃO dá — já traduzido a partir da chave de catálogo.
    pub motivo: String,
}

/// **O quê:** formata bytes em GB com uma casa. **Onde:** [`ler_diag`].
///
/// Vazio quando não se sabe, e **nunca `"0.0 GB"`**: uma máquina com zero de RAM não existe, e
/// mostrar zero faria a pessoa achar que a leitura deu certo e o número é esse.
fn gb(bytes: Option<f64>) -> String {
    match bytes {
        Some(b) if b > 0.0 => format!("{:.1} GB", b / 1_073_741_824.0),
        _ => String::new(),
    }
}

/// **O quê:** o número de uma chave, se for número. **Onde:** [`ler_diag`].
fn num(o: &Json, chave: &str) -> Option<f64> {
    match o.get(chave) {
        Some(Json::Num(n)) => Some(*n),
        _ => None,
    }
}

/// **O quê:** o nome do fabricante da GPU, a partir do slug estável do contrato.
///
/// **Onde:** [`ler_diag`].
///
/// **Conjunto FECHADO, e o `_` é honesto.** Um slug que esta janela não conhece significa que o
/// optimizer é mais novo que ela; mostrar o slug cru é melhor que mostrar vazio, porque ao
/// menos diz alguma coisa e a pessoa pode pesquisá-lo.
fn fabricante(slug: &str) -> String {
    match slug {
        "amd" => "AMD".into(),
        "intel" => "Intel".into(),
        "nvidia" => "NVIDIA".into(),
        "unknown" => "não identificada".into(),
        outro => outro.to_string(),
    }
}

/// **O quê:** traduz a chave de catálogo do motivo. **Onde:** [`ler_diag`].
///
/// **A tabela é pequena porque o conjunto é FECHADO** — são quatro motivos, e eles vivem no
/// `avaliar` do optimizer. Uma chave desconhecida cai no próprio texto da chave: feio, e ainda
/// assim melhor que um espaço em branco onde deveria haver a explicação.
fn motivo(chave: &str) -> String {
    match chave {
        "gpu.amd_no_gtt" => "a GPU é AMD, mas o driver não expõe o tamanho do GTT.".into(),
        "gpu.intel_firmware" => {
            "no Intel a memória da GPU é fixada pelo firmware (DVMT, no setup da UEFI) — \
             o Linux não a altera em execução."
                .into()
        }
        "gpu.nvidia_own_vram" => {
            "a NVIDIA discreta usa a própria VRAM; não há memória do sistema a limitar.".into()
        }
        "gpu.unknown_vendor" => "não consegui identificar a GPU desta máquina.".into(),
        outra => outra.to_string(),
    }
}

/// **O quê:** o modelo da tela de diagnóstico, a partir do `diag --json`.
///
/// **Onde:** a janela, ao abrir. PURA.
pub fn ler_diag(texto: &str) -> Result<Diag, String> {
    let j = json::ler(texto).map_err(|e| format!("resposta do optimizer ilegível: {e}"))?;
    let m = j.get("machine").cloned().unwrap_or(Json::Null);
    let g = j.get("gpu").cloned().unwrap_or(Json::Null);
    let c = j.get("vram_cap").cloned().unwrap_or(Json::Null);

    let cg = m.bool("cgroup_v2").unwrap_or(false);
    let limitavel = c.bool("limitable").unwrap_or(false);
    Ok(Diag {
        versao: j.str_ou_vazio("optimizer"),
        ram: gb(num(&m, "ram_bytes")),
        nucleos: num(&m, "cores").map(|n| format!("{n:.0}")).unwrap_or_default(),
        cgroups: if cg { "disponível".into() } else { "ausente".into() },
        cgroups_ok: cg,
        gpu: fabricante(&g.str_ou_vazio("vendor")),
        vram: gb(num(&g, "vram_bytes")),
        gtt: gb(num(&g, "gtt_bytes")),
        limitavel,
        parametro: c.str_ou_vazio("param"),
        atual: num(&c, "current_mib").map(|n| format!("{n:.0} MiB")).unwrap_or_default(),
        sugerido: num(&c, "suggested_mib").map(|n| format!("{n:.0} MiB")).unwrap_or_default(),
        // O motivo só é traduzido quando existe: um `null` aqui significa "dá para limitar", e
        // traduzir a palavra `null` poria "null" na tela.
        motivo: c.str("reason").map(motivo).unwrap_or_default(),
    })
}

// ---------------------------------------------------------------------------
// Tela 2 — LIMITES
// ---------------------------------------------------------------------------

/// Uma caixa (teto por software), com o que ela é e o que vai acontecer com ela.
#[derive(Debug, Clone, PartialEq)]
pub struct Caixa {
    pub nome: String,
    /// Para que serve, já traduzido da chave de catálogo.
    pub descricao: String,
    pub memoria_apertar: String,
    pub memoria_matar: String,
    pub cpu: String,
    pub io: String,
    /// Já está no disco?
    pub aplicada: bool,
}

/// O que a tela de limites mostra.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Limites {
    pub versao: String,
    pub cgroups_ok: bool,
    pub caixas: Vec<Caixa>,
    /// A FRASE que a CLI não dá: o que muda na máquina se a pessoa aplicar agora.
    pub diferenca: String,
    /// Há algo a aplicar? `false` quando o disco já bate com o sugerido.
    pub ha_mudanca: bool,
}

/// **O quê:** traduz a chave de catálogo da descrição de uma caixa. **Onde:** [`ler_limites`].
fn descricao(chave: &str) -> String {
    match chave {
        "box.build" => "compilação (cargo, make, npm) — o que mais infla a memória".into(),
        "box.browser" => "navegador — cresce sem parar e ninguém percebe".into(),
        "box.container" => "containers (docker, podman)".into(),
        outra => outra.to_string(),
    }
}

/// **O quê:** MiB em texto, ou vazio quando não se sabe. **Onde:** [`ler_limites`].
fn mib(o: &Json, chave: &str) -> String {
    match num(o, chave) {
        Some(n) => format!("{n:.0} MiB"),
        None => String::new(),
    }
}

/// **O quê:** a frase que diz o que MUDA na máquina — o que a CLI não dá.
///
/// **Onde:** [`ler_limites`], e é a razão de a tela existir.
///
/// **Ela nomeia as caixas, e não conta quantas.** "3 caixas serão criadas" obriga a pessoa a
/// ir procurar quais; nomeá-las custa a mesma linha e responde a pergunta que ela tem.
///
/// **"Nada muda" é resultado, não erro.** Quem chega numa máquina já configurada precisa
/// ouvir isso — e um botão de aplicar habilitado ali seria um convite a reescrever o que já
/// está certo.
fn diferenca(caixas: &[Caixa]) -> (String, bool) {
    let novas: Vec<&str> = caixas.iter().filter(|c| !c.aplicada).map(|c| c.nome.as_str()).collect();
    if novas.is_empty() {
        return ("Nada muda: os tetos sugeridos já estão aplicados nesta máquina.".into(), false);
    }
    let lista = novas.join(", ");
    let frase = if novas.len() == caixas.len() {
        format!("Vai criar {} teto(s), todos novos: {lista}.", novas.len())
    } else {
        format!("Vai criar {} teto(s): {lista}. Os demais já estão aplicados.", novas.len())
    };
    (frase, true)
}

/// **O quê:** o modelo da tela de limites, a partir do `limits --json`.
///
/// **Onde:** a janela, ao abrir a tela e depois de aplicar. PURA.
pub fn ler_limites(texto: &str) -> Result<Limites, String> {
    let j = json::ler(texto).map_err(|e| format!("resposta do optimizer ilegível: {e}"))?;
    let aplicadas: Vec<&str> = j.arr("applied").iter().filter_map(|x| x.como_str()).collect();
    let caixas: Vec<Caixa> = j
        .arr("boxes")
        .iter()
        .map(|o| {
            let nome = o.str_ou_vazio("name");
            Caixa {
                aplicada: aplicadas.contains(&nome.as_str()),
                descricao: descricao(&o.str_ou_vazio("description")),
                memoria_apertar: mib(o, "memory_high_mib"),
                memoria_matar: mib(o, "memory_max_mib"),
                // Sem cota de CPU é uma decisão, não uma falta: o build não leva teto de CPU
                // de propósito. Mostrar vazio faria parecer que o dado não foi lido.
                cpu: match num(o, "cpu_quota_pct") {
                    Some(n) => format!("{n:.0}%"),
                    None => "sem teto".into(),
                },
                io: match num(o, "io_weight") {
                    Some(n) => format!("{n:.0}"),
                    None => "padrão".into(),
                },
                nome,
            }
        })
        .collect();
    let (frase, ha) = diferenca(&caixas);
    Ok(Limites {
        versao: j.str_ou_vazio("optimizer"),
        cgroups_ok: j.bool("cgroup_v2").unwrap_or(false),
        caixas,
        diferenca: frase,
        ha_mudanca: ha,
    })
}

// ---------------------------------------------------------------------------
// Lista secundária — SERVIÇOS de boot
// ---------------------------------------------------------------------------

/// Um serviço de boot, como a lista secundária o mostra.
#[derive(Debug, Clone, PartialEq)]
pub struct Servico {
    pub unidade: String,
    pub custo: String,
    /// Está na allowlist auditada?
    pub auditado: bool,
    /// Vale a pena desligar NESTA máquina? Implica o anterior.
    pub recomendado: bool,
    pub explicacao: String,
}

/// **O quê:** traduz as chaves de catálogo da justificativa. **Onde:** [`ler_servicos`].
fn justificativa(faz: &str, perde: &str) -> String {
    let t = |c: &str| -> String {
        match c {
            "svc.nm_wait.does" => "espera a rede ficar pronta antes de o boot seguir".into(),
            "svc.nm_wait.why" => "numa máquina de dev nada no boot depende da rede".into(),
            "svc.nm_wait.lose" => {
                "serviços que exigem rede no boot podem subir antes de ela estar pronta".into()
            }
            "svc.appstream.does" => "atualiza o cache de metadados da loja de aplicativos".into(),
            "svc.appstream.why" => "quem instala pelo terminal não usa esse cache".into(),
            "svc.appstream.lose" => "a loja gráfica pode mostrar dados desatualizados".into(),
            outra => outra.to_string(),
        }
    };
    if faz.is_empty() {
        return String::new();
    }
    format!("{}. Custo de desligar: {}.", t(faz), t(perde))
}

/// **O quê:** os serviços de boot, a partir do `services --json`.
///
/// **Onde:** a lista secundária da tela de limites. PURA.
///
/// **`auditado` e `recomendado` são DOIS campos porque são duas afirmações.** "É seguro
/// desligar" é propriedade do serviço, auditada e justificada; "vale a pena desligar nesta
/// máquina" é medição. Um serviço da allowlist que não custa nada aqui continua seguro — só
/// não vale o trabalho, e a tela diz as duas coisas.
pub fn ler_servicos(texto: &str) -> Result<Vec<Servico>, String> {
    let j = json::ler(texto).map_err(|e| format!("resposta do optimizer ilegível: {e}"))?;
    Ok(j.arr("services")
        .iter()
        .map(|o| Servico {
            unidade: o.str_ou_vazio("unit"),
            custo: match num(o, "cost_s") {
                Some(c) => format!("{c:.2}s"),
                // Não apareceu no `blame`: não custou nada MEDÍVEL neste boot. É diferente de
                // "custou zero", e mostrar `0.00s` afirmaria o que não se mediu.
                None => "não medido".into(),
            },
            auditado: o.bool("on_allowlist").unwrap_or(false),
            recomendado: o.bool("recommended").unwrap_or(false),
            explicacao: justificativa(
                o.str("does").unwrap_or_default(),
                o.str("what_you_lose").unwrap_or_default(),
            ),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A saída real do `diag --json` desta máquina, copiada do terminal.
    const DIAG: &str = r#"{
  "optimizer": "0.4.0",
  "machine": { "ram_bytes": 16695111680, "cores": 8, "cgroup_v2": true },
  "gpu": { "vendor": "amd", "vram_bytes": 8589934592, "gtt_bytes": 8347553792 },
  "vram_cap": { "limitable": true, "param": "amdgpu.gttsize", "current_mib": 7960,
                "suggested_mib": 3980, "reason": null },
  "in_menu": true
}"#;

    /// A saída real do `limits --json`, com o disco VAZIO (nada aplicado ainda).
    const LIMITES: &str = r#"{
  "optimizer": "0.4.0",
  "cgroup_v2": true,
  "boxes": [
    {"name": "dev-build", "description": "box.build", "memory_high_mib": 7960, "memory_max_mib": 11940, "cpu_quota_pct": null, "io_weight": 50},
    {"name": "dev-navegador", "description": "box.browser", "memory_high_mib": 3980, "memory_max_mib": 5970, "cpu_quota_pct": 200, "io_weight": 100},
    {"name": "dev-container", "description": "box.container", "memory_high_mib": 3980, "memory_max_mib": 5970, "cpu_quota_pct": 700, "io_weight": 80}
  ],
  "applied": []
}"#;

    /// Os números da tela batem com os do contrato — e nenhum deles é inventado.
    #[test]
    fn o_diagnostico_le_o_contrato_inteiro() {
        let d = ler_diag(DIAG).unwrap();
        assert_eq!(d.versao, "0.4.0");
        assert_eq!(d.ram, "15.5 GB");
        assert_eq!(d.nucleos, "8");
        assert!(d.cgroups_ok);
        assert_eq!(d.gpu, "AMD");
        assert_eq!(d.vram, "8.0 GB");
        assert!(d.limitavel);
        assert_eq!(d.parametro, "amdgpu.gttsize");
        assert_eq!(d.atual, "7960 MiB");
        assert_eq!(d.sugerido, "3980 MiB");
        assert_eq!(d.motivo, "", "dá para limitar: não há motivo a mostrar");
    }

    /// **O motivo é traduzido da CHAVE, e a chave nunca chega à tela quando é conhecida.**
    /// O contrato não carrega prosa de propósito — é o que o mantém igual em qualquer idioma.
    #[test]
    fn o_motivo_vem_da_chave_e_nao_do_contrato() {
        let intel = DIAG
            .replace(r#""limitable": true"#, r#""limitable": false"#)
            .replace(r#""reason": null"#, r#""reason": "gpu.intel_firmware""#)
            .replace(r#""vendor": "amd""#, r#""vendor": "intel""#);
        let d = ler_diag(&intel).unwrap();
        assert!(!d.limitavel);
        assert_eq!(d.gpu, "Intel");
        assert!(d.motivo.contains("firmware"), "{}", d.motivo);
        assert!(!d.motivo.contains("gpu."), "a chave crua não pode chegar à tela: {}", d.motivo);
    }

    /// Chave de motivo desconhecida (optimizer mais novo que a janela) cai no texto da chave —
    /// feio, e ainda assim melhor que um espaço em branco onde deveria haver a explicação.
    #[test]
    fn motivo_desconhecido_mostra_a_chave_em_vez_de_nada() {
        let novo = DIAG.replace(r#""reason": null"#, r#""reason": "gpu.motivo_que_nao_existe""#);
        let d = ler_diag(&novo).unwrap();
        assert_eq!(d.motivo, "gpu.motivo_que_nao_existe");
        assert!(!d.motivo.is_empty());
    }

    /// **Zero não é "não sei".** RAM zero não existe; mostrar `0.0 GB` faria a pessoa achar
    /// que a leitura deu certo e o número é esse.
    #[test]
    fn valor_ausente_fica_vazio_e_nao_vira_zero() {
        let d = ler_diag(r#"{"machine":{"ram_bytes":0},"gpu":{"vram_bytes":null}}"#).unwrap();
        assert_eq!(d.ram, "");
        assert_eq!(d.vram, "");
        assert_eq!(d.gtt, "");
    }

    /// **A FRASE que a CLI não dá.** É a razão de a tela de limites existir: entre "isto é o
    /// que eu sugeriria" e "escrevi", a CLI não tem nada.
    #[test]
    fn a_tela_diz_o_que_muda_antes_de_aplicar() {
        let l = ler_limites(LIMITES).unwrap();
        assert!(l.ha_mudanca);
        // Nomeia as caixas: "3 caixas serão criadas" obriga a pessoa a ir procurar quais.
        for nome in ["dev-build", "dev-navegador", "dev-container"] {
            assert!(l.diferenca.contains(nome), "{}", l.diferenca);
        }
        assert!(l.caixas.iter().all(|c| !c.aplicada));
    }

    /// **"Nada muda" é RESULTADO, não erro.** Quem chega numa máquina já configurada precisa
    /// ouvir isso, e um botão de aplicar habilitado ali convidaria a reescrever o que já está
    /// certo.
    #[test]
    fn maquina_ja_configurada_diz_que_nada_muda() {
        let tudo = LIMITES.replace(
            r#""applied": []"#,
            r#""applied": ["dev-build", "dev-navegador", "dev-container"]"#,
        );
        let l = ler_limites(&tudo).unwrap();
        assert!(!l.ha_mudanca, "nada a aplicar");
        assert!(l.diferenca.contains("Nada muda"), "{}", l.diferenca);
        assert!(l.caixas.iter().all(|c| c.aplicada));
    }

    /// Parcialmente aplicado: a frase nomeia só o que falta, e diz que o resto já está.
    #[test]
    fn parcialmente_aplicado_nomeia_so_o_que_falta() {
        let meio = LIMITES.replace(r#""applied": []"#, r#""applied": ["dev-build"]"#);
        let l = ler_limites(&meio).unwrap();
        assert!(l.ha_mudanca);
        assert!(!l.diferenca.contains("dev-build"), "já aplicado: {}", l.diferenca);
        assert!(l.diferenca.contains("dev-navegador"), "{}", l.diferenca);
        assert!(l.diferenca.contains("já estão aplicados"), "{}", l.diferenca);
        assert!(l.caixas[0].aplicada);
        assert!(!l.caixas[1].aplicada);
    }

    /// **"Sem teto" é uma DECISÃO, não uma falta.** O build não leva cota de CPU de propósito
    /// — um build lento por cota é o oposto do que o app se propõe. Mostrar vazio faria
    /// parecer que o dado não foi lido.
    #[test]
    fn sem_cota_de_cpu_e_dito_e_nao_deixado_em_branco() {
        let l = ler_limites(LIMITES).unwrap();
        assert_eq!(l.caixas[0].cpu, "sem teto");
        assert_eq!(l.caixas[1].cpu, "200%");
    }

    /// A descrição é traduzida da chave; a chave crua não chega à tela.
    #[test]
    fn a_descricao_vem_da_chave_de_catalogo() {
        let l = ler_limites(LIMITES).unwrap();
        assert!(l.caixas[0].descricao.contains("compilação"), "{:?}", l.caixas[0].descricao);
        assert!(!l.caixas[0].descricao.starts_with("box."), "{:?}", l.caixas[0].descricao);
    }

    /// **Auditado e recomendado são coisas diferentes**, e a lista precisa das duas: um
    /// serviço da allowlist que não custa nada aqui continua seguro, só não vale o trabalho.
    #[test]
    fn auditado_e_recomendado_sao_afirmacoes_diferentes() {
        let s = ler_servicos(
            r#"{"services":[
              {"unit":"a.service","cost_s":5.14,"on_allowlist":true,"recommended":true,
               "does":"svc.nm_wait.does","why_safe":"svc.nm_wait.why","what_you_lose":"svc.nm_wait.lose"},
              {"unit":"b.service","cost_s":null,"on_allowlist":true,"recommended":false,
               "does":"svc.appstream.does","why_safe":"svc.appstream.why","what_you_lose":"svc.appstream.lose"},
              {"unit":"c.service","cost_s":0.42,"on_allowlist":false,"recommended":false,
               "does":null,"why_safe":null,"what_you_lose":null}]}"#,
        )
        .unwrap();
        assert_eq!(s.len(), 3);
        assert!(s[0].auditado && s[0].recomendado);
        assert!(s[1].auditado && !s[1].recomendado, "seguro, mas não custa nada aqui");
        assert!(!s[2].auditado && !s[2].recomendado);
        // A justificativa continua lá para o item seguro-mas-barato: escondê-la tiraria da
        // pessoa o que ela precisa para decidir.
        assert!(!s[1].explicacao.is_empty());
        assert!(s[2].explicacao.is_empty(), "fora da allowlist não há o que justificar");
        // E ela é traduzida, não a chave crua.
        assert!(!s[0].explicacao.contains("svc."), "{}", s[0].explicacao);
    }

    /// Custo não medido é dito, e não vira `0.00s` — afirmar zero é afirmar o que não se mediu.
    #[test]
    fn custo_nao_medido_e_dito_e_nao_vira_zero() {
        let s = ler_servicos(r#"{"services":[{"unit":"a.service","cost_s":null}]}"#).unwrap();
        assert_eq!(s[0].custo, "não medido");
    }

    /// **Nada aqui pode entrar em pânico.** Uma janela que morre ao abrir é pior que uma tela
    /// vazia: a pessoa não vê nem a mensagem de erro.
    #[test]
    fn entrada_hostil_nao_panica() {
        // Documento válido e vazio: modelo vazio, e isso é um fato.
        assert_eq!(ler_diag("{}").unwrap(), Diag { cgroups: "ausente".into(), ..Diag::default() });
        assert!(ler_limites("{}").unwrap().caixas.is_empty());
        assert!(ler_servicos("{}").unwrap().is_empty());
        // Tipos trocados: campos vazios, nunca pânico.
        assert!(ler_diag(r#"{"machine":"nao e objeto","gpu":[1,2]}"#).is_ok());
        assert!(ler_limites(r#"{"boxes":"nao e lista","applied":1}"#).is_ok());
        // Ilegível é Err — tela vazia sem erro afirmaria que não há nada.
        for lixo in ["", "isto nao e json", r#"{"boxes":["#] {
            assert!(ler_diag(lixo).is_err(), "{lixo:?}");
            assert!(ler_limites(lixo).is_err(), "{lixo:?}");
            assert!(ler_servicos(lixo).is_err(), "{lixo:?}");
        }
    }

    /// **O modelo não muda de idioma.** O contrato do optimizer não carrega prosa, então trocar
    /// o idioma do app não pode mover um número nem um estado desta tela.
    #[test]
    fn o_modelo_nao_depende_do_idioma_do_app() {
        // O contrato é o mesmo em qualquer idioma — é o que o teste de byte-identidade do
        // optimizer garante do lado de lá. Aqui a prova é que nada nesta leitura olha prosa:
        // se olhasse, um documento com prosa acrescentada mudaria o resultado.
        let com_prosa = DIAG.replace(
            r#""in_menu": true"#,
            r#""in_menu": true, "mensagem_humana": "プラットフォーム""#,
        );
        assert_eq!(ler_diag(&com_prosa).unwrap(), ler_diag(DIAG).unwrap());
    }
}

// ============================ ABA DISCO ============================
//
// O inventário do lixo RECRIÁVEL, lido do `disco --json`. Ele saiu do hub na E3 da extradição
// (ADR-0011/0018): enquanto morava lá, existiam DUAS telas do mesmo produto.

/// Um total agregado — serve às duas visões de cima da aba (por disco e por tipo).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TotalDisco {
    /// O ponto de montagem, ou o tipo de artefato — depende da visão.
    pub chave: String,
    pub bytes: u64,
    /// `true` quando refazer isto **baixa da rede** de novo. Só faz sentido por TIPO.
    pub custa_rede: bool,
}

/// Um achado: uma pasta concreta que pode ser apagada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Achado {
    pub caminho: String,
    pub tipo: String,
    pub bytes: u64,
    pub dias_parado: i64,
    pub montagem: String,
    /// Como isto se refaz, em uma linha. **Vem pronto do app**, e não é montado aqui: é o
    /// mesmo texto que o terminal imprime, e duas versões dele divergiriam.
    pub refaz: String,
}

/// O que a aba Disco mostra.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Disco {
    pub total_bytes: u64,
    pub por_montagem: Vec<TotalDisco>,
    pub por_tipo: Vec<TotalDisco>,
    pub achados: Vec<Achado>,
}

/// **O quê:** lê o `disco --json`.
///
/// **Onde:** a aba Disco. PURA.
///
/// **Número negativo ou ausente vira ZERO, e nunca um tamanho inventado.** A frase desta tela
/// é "isto pode ser apagado"; um número errado ali é um convite a apagar a coisa errada.
pub fn ler_disco(texto: &str) -> Result<Disco, String> {
    let j = json::ler(texto).map_err(|e| format!("resposta do optimizer ilegível: {e}"))?;
    let totais = |chave: &str| -> Vec<TotalDisco> {
        j.arr(chave)
            .iter()
            .map(|t| TotalDisco {
                // A chave tem nome diferente nas duas listas, e é o mesmo papel na tela.
                chave: if chave == "por_montagem" {
                    t.str_ou_vazio("montagem")
                } else {
                    t.str_ou_vazio("tipo")
                },
                bytes: bytes_de(t, "bytes"),
                custa_rede: t.bool("custa_rede") == Some(true),
            })
            .collect()
    };
    Ok(Disco {
        total_bytes: bytes_de(&j, "total_bytes"),
        por_montagem: totais("por_montagem"),
        por_tipo: totais("por_tipo"),
        achados: j
            .arr("achados")
            .iter()
            .map(|a| Achado {
                caminho: a.str_ou_vazio("caminho"),
                tipo: a.str_ou_vazio("tipo"),
                bytes: bytes_de(a, "bytes"),
                dias_parado: a.num("dias_parado").filter(|n| *n >= 0.0).unwrap_or(0.0) as i64,
                montagem: a.str_ou_vazio("montagem"),
                refaz: a.str_ou_vazio("refaz_text"),
            })
            .collect(),
    })
}

/// **O quê:** um campo de bytes, nunca negativo. **Onde:** [`ler_disco`].
fn bytes_de(j: &json::Json, chave: &str) -> u64 {
    j.num(chave).filter(|n| n.is_finite() && *n >= 0.0).unwrap_or(0.0) as u64
}

/// **O quê:** bytes em texto curto (`6,3 GB`).
///
/// **Onde:** a aba Disco.
///
/// **Base 1000 e não 1024**, porque é o que o fabricante do disco e o gerenciador de arquivos
/// do sistema usam. Mostrar GiB aqui faria o total da tela não bater com o que a pessoa vê no
/// resto da máquina, e ela concluiria que uma das duas contas está errada.
pub fn tamanho(bytes: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1000.0 && i + 1 < U.len() {
        v /= 1000.0;
        i += 1;
    }
    if i == 0 {
        return format!("{bytes} B");
    }
    format!("{v:.1} {}", U[i])
}

// ============================ ABA AGENTES ============================

/// Quantos agentes esta máquina aguenta em paralelo, e por quê.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Agentes {
    pub threads: i64,
    pub mem_available_mb: i64,
    pub load1: f64,
    pub running: i64,
    pub cpu_cap: i64,
    pub ram_cap: i64,
    pub load_cap: i64,
    /// O menor dos três tetos — é ele que vale.
    pub total_cap: i64,
    /// Quantos ainda cabem AGORA. Nunca negativo.
    pub available: i64,
    pub ram_tight: bool,
}

impl Agentes {
    /// **O quê:** qual dos três tetos é o que está mandando.
    ///
    /// **Onde:** a aba Agentes, para dizer O QUE limitar se a pessoa quiser mais.
    ///
    /// **Existe porque "cabem 4" sozinho não diz nada acionável.** Saber que o teto é de RAM e
    /// não de CPU é a diferença entre fechar o navegador e comprar um processador.
    pub fn gargalo(&self) -> &'static str {
        if self.total_cap == self.ram_cap && self.ram_cap <= self.cpu_cap.min(self.load_cap) {
            return "memória";
        }
        if self.total_cap == self.cpu_cap && self.cpu_cap <= self.load_cap {
            return "núcleos";
        }
        if self.total_cap == self.load_cap {
            return "carga atual da máquina";
        }
        "—"
    }
}

/// **O quê:** lê o `agentes --json`.
///
/// **Onde:** a aba Agentes. PURA.
///
/// **`available` nunca fica negativo.** Rodando 5 com teto 4, a conta dá -1; mostrar "-1
/// disponíveis" é uma frase que ninguém sabe ler. Zero, e a tela diz que já passou do teto.
pub fn ler_agentes(texto: &str) -> Result<Agentes, String> {
    let j = json::ler(texto).map_err(|e| format!("resposta do optimizer ilegível: {e}"))?;
    let n = |c: &str| -> i64 { j.num(c).filter(|v| v.is_finite()).unwrap_or(0.0) as i64 };
    Ok(Agentes {
        threads: n("threads"),
        mem_available_mb: n("mem_available_mb"),
        load1: j.num("load1").filter(|v| v.is_finite() && *v >= 0.0).unwrap_or(0.0),
        running: n("running_claudes"),
        cpu_cap: n("cpu_cap"),
        ram_cap: n("ram_cap"),
        load_cap: n("load_cap"),
        total_cap: n("total_cap"),
        available: n("available").max(0),
        ram_tight: j.bool("ram_tight") == Some(true),
    })
}

#[cfg(test)]
mod tests_disco_agentes {
    use super::*;

    const DISCO: &str = r#"{"optimizer":"0.4.0","total_bytes":7544807424,
      "por_montagem":[{"montagem":"/home","bytes":7544807424}],
      "por_tipo":[{"tipo":"go-cache","bytes":6264434688,"custa_rede":true},
                  {"tipo":"cargo-cache","bytes":768831488,"custa_rede":false}],
      "achados":[{"caminho":"/home/u/go/pkg/mod","tipo":"go-cache","bytes":5725962240,
                  "dias_parado":15,"montagem":"/home","refaz_text":"baixado de novo no go build"}]}"#;

    #[test]
    fn le_o_disco() {
        let d = ler_disco(DISCO).expect("lê");
        assert_eq!(d.total_bytes, 7_544_807_424);
        assert_eq!(d.por_montagem[0].chave, "/home", "a chave da visão por montagem é o mount");
        assert_eq!(d.por_tipo[0].chave, "go-cache", "e a da visão por tipo é o tipo");
        assert!(d.por_tipo[0].custa_rede);
        assert!(!d.por_tipo[1].custa_rede);
        assert_eq!(d.achados[0].dias_parado, 15);
        assert_eq!(d.achados[0].refaz, "baixado de novo no go build");
    }

    /// **Número ausente, negativo ou de tipo errado vira ZERO — nunca um tamanho inventado.**
    ///
    /// A frase desta tela é "isto pode ser apagado". Um número errado ali é um convite a apagar
    /// a coisa errada, e o valor coagido é indistinguível do medido depois que entra.
    #[test]
    fn tamanho_ruim_nao_vira_afirmacao() {
        let d = ler_disco(
            r#"{"total_bytes":"muitos","achados":[
              {"caminho":"/a","bytes":-5,"dias_parado":-3},
              {"caminho":"/b"}]}"#,
        )
        .expect("lê");
        assert_eq!(d.total_bytes, 0, "texto não vira número");
        assert_eq!(d.achados[0].bytes, 0, "negativo não vira tamanho");
        assert_eq!(d.achados[0].dias_parado, 0);
        assert_eq!(d.achados[1].bytes, 0, "campo ausente é zero, não lixo");
    }

    /// **Base 1000, como o disco e o gerenciador de arquivos.** Em GiB o total desta tela não
    /// bateria com o que a pessoa vê no resto da máquina.
    #[test]
    fn o_tamanho_usa_a_mesma_base_do_sistema() {
        assert_eq!(tamanho(0), "0 B");
        assert_eq!(tamanho(999), "999 B");
        assert_eq!(tamanho(1_000), "1.0 KB");
        assert_eq!(tamanho(7_544_807_424), "7.5 GB");
        // Em base 1024 isto daria 7.0 GiB — e a pessoa acharia que uma das contas mente.
        assert_ne!(tamanho(7_544_807_424), "7.0 GB");
    }

    #[test]
    fn le_os_agentes() {
        let a = ler_agentes(
            r#"{"threads":8,"mem_available_mb":9313,"load1":1.95,"running_claudes":3,
                "cpu_cap":4,"ram_cap":7,"load_cap":6,"total_cap":4,"available":1,
                "ram_tight":false}"#,
        )
        .expect("lê");
        assert_eq!((a.threads, a.total_cap, a.available), (8, 4, 1));
        assert_eq!(a.gargalo(), "núcleos", "o teto que manda é o de CPU");
    }

    /// **O gargalo é a informação ACIONÁVEL.** "Cabem 4" sozinho não diz nada; saber que o
    /// teto é de RAM e não de CPU é a diferença entre fechar o navegador e trocar de máquina.
    #[test]
    fn o_gargalo_nomeia_o_teto_que_manda() {
        let ler = |s: &str| ler_agentes(s).expect("lê");
        let a = ler(r#"{"cpu_cap":8,"ram_cap":2,"load_cap":6,"total_cap":2}"#);
        assert_eq!(a.gargalo(), "memória");
        let a = ler(r#"{"cpu_cap":8,"ram_cap":9,"load_cap":1,"total_cap":1}"#);
        assert_eq!(a.gargalo(), "carga atual da máquina");
    }

    /// **`available` nunca fica negativo.** Rodando 5 com teto 4 a conta dá -1, e "-1
    /// disponíveis" é uma frase que ninguém sabe ler.
    #[test]
    fn nunca_ha_menos_de_zero_disponivel() {
        let a = ler_agentes(r#"{"total_cap":4,"running_claudes":5,"available":-1}"#).expect("lê");
        assert_eq!(a.available, 0);
    }

    /// Entrada hostil não panica — janela que morre ao abrir é pior que tela vazia.
    #[test]
    fn entrada_hostil_nao_panica() {
        let fundo = format!("{}{}", "[".repeat(3000), "]".repeat(3000));
        for lixo in
            ["", "null", "[]", "0", "\"t\"", "\u{0}", &fundo, r#"{"achados":"nao e lista"}"#]
        {
            let _ = ler_disco(lixo);
            let _ = ler_agentes(lixo);
        }
    }
}
