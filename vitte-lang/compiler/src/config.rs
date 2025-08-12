use camino::Utf8PathBuf;

/// Configuration du compilateur.
#[derive(Debug, Clone)]
pub struct CompilerConfig {
    /// Chemin du binaire `vitte` (si non fourni: $VITTE_BIN ou 'vitte' dans PATH)
    pub vitte_bin: Option<Utf8PathBuf>,
    /// Dossier de travail (pour artefacts temporaires)
    pub workdir: Utf8PathBuf,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            vitte_bin: std::env::var("VITTE_BIN").ok().map(Into::into),
            workdir: Utf8PathBuf::from(std::env::temp_dir().to_string_lossy().to_string()),
        }
    }
}