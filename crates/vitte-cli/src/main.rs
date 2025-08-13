//! vitte-cli/src/main.rs
//!
//! Point d’entrée du binaire `vitte`.
//! Ici, on se contente de préparer l’environnement (logs, rapports d’erreurs)
//! puis on délègue toute la logique à `vitte_cli::run()`.
//!
//! Avantages :
//! - `lib.rs` peut être testé en unité (cargo test -p vitte-cli)
//! - main.rs reste minimal, juste pour le setup global

fn main() {
    // 📌 Initialisation des rapports d’erreurs stylés
    if let Err(e) = color_eyre::install() {
        eprintln!("⚠️ Impossible d'initialiser color-eyre: {e}");
    }

    // 📌 Optionnel : activer les logs si RUST_LOG est défini
    env_logger::init();

    // 📌 Lancer le cœur du CLI
    if let Err(err) = vitte_cli::run() {
        eprintln!("❌ Erreur: {err}");

        // 📌 Astuce: affiche un backtrace si l’utilisateur a activé RUST_BACKTRACE=1
        if std::env::var("RUST_BACKTRACE").as_deref() == Ok("1") {
            if let Some(bt) = err.backtrace() {
                eprintln!("\n📜 Backtrace:\n{bt}");
            }
        }

        std::process::exit(1);
    }
}
