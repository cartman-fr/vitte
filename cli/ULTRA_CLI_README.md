# vitte-lang • Ultra CLI pack

Ce pack contient:
- `src/main.rs` (routing Clap v4, flags VM globaux)
- `src/commands/` complet
- `src/commands/completions.rs` (bash/zsh/fish/powershell/elvish)
- `src/commands/man.rs` (page man)

## Dépendances (Cargo.toml)
```toml
[dependencies]
clap = { version = "4.5", features = ["derive"] }
clap_complete = "4.5"
clap_mangen = "0.2"
color-eyre = "0.6"   # déjà utilisé dans le projet
```
## Build
```bash
cargo build --release
./target/release/vitte --help
./target/release/vitte completions --shell zsh > _vitte
./target/release/vitte man > vitte.1
```