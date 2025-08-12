
use color_eyre::eyre::{Result, eyre};
use std::path::{Path, PathBuf};
use std::collections::HashSet;

pub fn read(path: &Path) -> Result<String> { Ok(std::fs::read_to_string(path)?) }
pub fn write(path: &Path, s: &str) -> Result<()> { if let Some(p)=path.parent(){ std::fs::create_dir_all(p)?; } std::fs::write(path, s)?; Ok(()) }
pub fn basename(path: &Path) -> String { path.file_name().and_then(|s| s.to_str()).unwrap_or("out").to_string() }
pub fn change_ext(path: &Path, ext: &str) -> PathBuf { let mut p=path.to_path_buf(); p.set_extension(ext); p }

/// Très simple résolveur d'imports : lignes `import foo` ⇒ lit `foo.vitte` (cwd ou ./modules).
/// Empile récursivement et déduplique par identifiant.
pub fn resolve_imports(main_src: &str, cwd: &Path) -> Result<String> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut out = String::new();
    fn load(id: &str, cwd: &Path, visited: &mut HashSet<String>, out: &mut String) -> Result<()> {
        if !visited.insert(id.to_string()) { return Ok(()); }
        let cand1 = cwd.join(format!("{}.vitte", id));
        let cand2 = cwd.join("modules").join(format!("{}.vitte", id));
        let path = if cand1.exists() { cand1 } else { cand2 };
        let s = std::fs::read_to_string(&path).map_err(|e| eyre!("import `{}` introuvable: {}", id, e))?;
        // Prétraitement récursif
        for line in s.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("import ") {
                let id2 = rest.trim();
                load(id2, path.parent().unwrap_or(cwd), visited, out)?;
            }
        }
        out.push_str(&s);
        out.push_str("\n");
        Ok(())
    }
    // 1) Charger imports depuis main_src
    for line in main_src.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("import ") {
            let id = rest.trim();
            load(id, cwd, &mut visited, &mut out)?;
        }
    }
    // 2) Ajouter le code principal ensuite
    out.push_str(main_src);
    Ok(out)
}
