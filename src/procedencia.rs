//! De qual commit este binário foi construído.
//!
//! **O quê:** casca fina sobre `schematize_plataforma::procedencia` (decisão pura e testes moram
//! na lib); aqui ficam só os NOMES das variáveis que o `build.rs` desta janela emite.
//!
//! **Onde:** [`rotulo_versao`] alimenta o `--version` da janela (`main.rs`).

/// **O quê:** o rótulo de versão deste binário (`&'static str`), pronto para o `--version`.
///
/// **Onde:** `main.rs`, no `--version` da janela.
pub fn rotulo_versao() -> &'static str {
    schematize_plataforma::rotulo_versao!("SCHEMATIZE_OPTGUI_SHA", "SCHEMATIZE_OPTGUI_DIRTY")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// O valor real deste build é um dos dois estados — nunca uma terceira coisa.
    #[test]
    fn o_build_de_verdade_produz_um_dos_dois_estados() {
        let r = rotulo_versao();
        assert!(r.starts_with(env!("CARGO_PKG_VERSION")), "{r}");
        assert!(r.ends_with(')') && r.contains(" ("), "procedência entre parênteses: {r}");
    }
}
