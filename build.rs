//! Compila `ui/optimizer.slint` → Rust (via `slint::include_modules!()` no main).
//! Mantém o .slint como arquivo de verdade (LSP do Slint, diff limpo).

fn main() {
    // O SHA do commit, para o `--version` dizer de QUAL build ele é. Sem isto o número sozinho
    // não distingue dois binários com o mesmo `Cargo.toml` — e foi assim que um binário de 15
    // dias atrás passou por novo e gravou o `.desktop` errado.
    let sha = std::process::Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    println!("cargo:rustc-env=SCHEMATIZE_OPTGUI_SHA={sha}");
    // Sem isto o valor congela no primeiro build e a versão passa a MENTIR.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");

    slint_build::compile("ui/optimizer.slint").expect("falha ao compilar ui/optimizer.slint");
}
