//! Orçamento de memória da janela (BUDGET.md): abre a GUI REAL, deixa desenhar a primeira tela e
//! confere o pico de RSS (`VmHWM` de `/proc/<pid>/status`) contra o teto.
//!
//! **O quê:** std-only. Sobe o binário, polla `VmHWM` até `JANELA` ou até o processo sair, mata a
//! janela e compara com [`TETO_KIB`]. **Onde:** gate local (`cargo test`).
//!
//! **Pula COM AVISO** (nunca `#[ignore]` mudo) sem `/proc` ou sem display (`DISPLAY` e
//! `WAYLAND_DISPLAY` vazios): sem display a janela nem abre, e não há o que medir.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Teto de RSS de pico da janela, em KiB: medido num build de DEBUG (30 MiB) + ~30%; o release
/// instalado mede ~17 MiB (BUDGET.md). Abaixo dos 400 MiB da casa.
const TETO_KIB: u64 = 40 * 1024;

/// Quanto tempo a janela fica aberta sendo medida.
const JANELA: Duration = Duration::from_secs(5);

/// **O quê:** lê o `VmHWM` (KiB) de `/proc/<pid>/status`. **Onde:** o laço de [`rss_de_pico_da_janela`].
fn vm_hwm_kib(pid: u32) -> Option<u64> {
    let s = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let linha = s.lines().find(|l| l.starts_with("VmHWM:"))?;
    linha.split_whitespace().nth(1)?.parse().ok()
}

/// A janela abre e o pico de RSS fica abaixo do teto.
#[test]
fn rss_de_pico_da_janela() {
    let sem_display = std::env::var_os("DISPLAY").is_none_or(|v| v.is_empty())
        && std::env::var_os("WAYLAND_DISPLAY").is_none_or(|v| v.is_empty());
    if sem_display || !std::path::Path::new("/proc/self/status").exists() {
        eprintln!("PULEI orcamento da janela: sem display ou sem /proc (nada a medir)");
        return;
    }
    let home = std::env::temp_dir().join(format!("optgui-orcamento-{}", std::process::id()));
    std::fs::create_dir_all(&home).expect("tmp");
    let mut filho = Command::new(env!("CARGO_BIN_EXE_schematize-optimizer-gui"))
        .env("HOME", &home)
        .env("SCHEMATIZE_SEM_MARKET", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("sobe a janela");
    let inicio = Instant::now();
    let mut pico = 0u64;
    let mut saiu_cedo = false;
    while inicio.elapsed() < JANELA {
        if let Ok(Some(_)) = filho.try_wait() {
            saiu_cedo = true;
            break;
        }
        if let Some(k) = vm_hwm_kib(filho.id()) {
            pico = pico.max(k);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = filho.kill();
    let _ = filho.wait();
    let _ = std::fs::remove_dir_all(&home);
    if saiu_cedo && pico == 0 {
        eprintln!(
            "PULEI orcamento da janela: o processo saiu antes da 1a leitura (display recusou?)"
        );
        return;
    }
    eprintln!("orcamento janela: pico {pico} KiB (teto {TETO_KIB} KiB)");
    assert!(pico > 1024, "pico {pico} KiB implausível: medidor cego");
    assert!(pico <= TETO_KIB, "RSS de pico {pico} KiB passou do teto {TETO_KIB} KiB");
}
