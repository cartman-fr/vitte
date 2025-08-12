
use std::fs;
use std::path::PathBuf;
use std::env;

fn registry_path() -> PathBuf {
    PathBuf::from("registry/index.json")
}

fn vendor_dir() -> PathBuf {
    PathBuf::from("vendor")
}

fn search(term: &str) {
    let idx = fs::read_to_string(registry_path()).expect("read registry");
    let v: serde_json::Value = serde_json::from_str(&idx).unwrap();
    let pkgs = v["packages"].as_array().unwrap();
    for p in pkgs {
        let name = p["name"].as_str().unwrap();
        if name.contains(term) {
            println!("{} {}", name, p["vers"].as_str().unwrap());
        }
    }
}

fn install(name: &str) {
    let idx = fs::read_to_string(registry_path()).expect("read registry");
    let v: serde_json::Value = serde_json::from_str(&idx).unwrap();
    let pkgs = v["packages"].as_array().unwrap();
    fs::create_dir_all(vendor_dir()).ok();
    for p in pkgs {
        if p["name"].as_str().unwrap() == name {
            let fname = format!("{}-{}.vpkg", name, p["vers"].as_str().unwrap());
            let path = vendor_dir().join(&fname);
            fs::write(&path, b"dummy package").unwrap();
            println!("installed {}", path.display());
            return;
        }
    }
    eprintln!("not found: {}", name);
}

fn usage() {
    eprintln!("vitpm search <term> | install <name>");
}

fn main(){
    let mut args: Vec<String> = env::args().collect();
    if args.len() < 2 { usage(); return; }
    match args[1].as_str() {
        "search" if args.len() >=3 => search(&args[2]),
        "install" if args.len() >=3 => install(&args[2]),
        _ => usage(),
    }
}
