// build.rs (à la racine du crate)
use std::path::PathBuf;

fn main() {
    // Imprime des conseils post-install propres (affichés en fin de build)
    let home = dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("~"));
    let bash_dir = home.join(".local/share/bash-completion/completions");
    let zsh_dir  = home.join(".zsh/completions");
    let fish_dir = home.join(".config/fish/completions");
    let pwsh_dir = home.join(".config/powershell/completions");
    let elv_dir  = home.join(".config/elvish/lib");

    println!("cargo:warning=────────────────────────────────────────────────────────");
    println!("cargo:warning=  ✅ vitte installé ! Ajoute l’auto-complétion en 1 commande :");
    println!("cargo:warning=    vitte completions --install");
    println!("cargo:warning=  Ou génère pour un shell précis :");
    println!("cargo:warning=    vitte completions --shell bash --dir {}", bash_dir.display());
    println!("cargo:warning=    vitte completions --shell zsh  --dir {}", zsh_dir.display());
    println!("cargo:warning=    vitte completions --shell fish --dir {}", fish_dir.display());
    println!("cargo:warning=    vitte completions --shell powershell --dir {}", pwsh_dir.display());
    println!("cargo:warning=    vitte completions --shell elvish --dir {}", elv_dir.display());
    println!("cargo:warning=────────────────────────────────────────────────────────");
}
