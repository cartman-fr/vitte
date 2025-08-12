use clap::{Parser, Subcommand};
use std::env;

pub mod commands;

#[derive(Parser)]
#[command(name = "vitte", version, about = "vitte-lang CLI ultra-complet", long_about = None)]
pub struct Cli {
    /// Verbosité (répéter pour plus de bruit)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Tracer la VM (équiv. VITTE_TRACE=1)
    #[arg(long)]
    trace: bool,

    /// Step-by-step VM (équiv. VITTE_TRACE_STEPS=1)
    #[arg(long)]
    trace_steps: bool,

    /// Breakpoints VM (ex: "12,27") (équiv. VITTE_BREAK)
    #[arg(long)]
    break_ip: Option<String>,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Compiler via vitte-compiler (délègue à `vitte bc`)
    Compile(commands::compile::Args),
    /// Exécuter un fichier .vitte via l’interpréteur
    Run(commands::run::Args),

    /// Compiler en bytecode .vbc
    Bc(commands::bc::Args),

    /// Exécuter un .vbc avec la VM
    Vm(commands::vm::Args),

    /// REPL interactive
    Repl(commands::repl::Args),

    /// Formatter le code
    Fmt(commands::fmt::Args),

    /// Lancer les tests (# EXPECT:)
    Tests(commands::tests::Args),

    /// Micro-benchs
    Bench(commands::bench::Args),

    /// Générer la doc statique (wiki/offline)
    Doc(commands::doc::Args),

    /// Passerelle codegen (LLVM / skeleton)
    Llc(commands::llc::Args),

    /// Package manager minimal (squelette)
    Pm(commands::pm::Args),

    /// Générer autocomplétions shell
    Completions(commands::completions::Args),

    /// Générer la page man
    Man(commands::man::Args),
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    // Propager flags VM en variables d’env (pour la VM bytecode)
    if cli.trace { env::set_var("VITTE_TRACE", "1"); }
    if cli.trace_steps { env::set_var("VITTE_TRACE_STEPS", "1"); }
    if let Some(b) = cli.break_ip.as_ref() { env::set_var("VITTE_BREAK", b); }

    // Verbosité
    if cli.verbose > 0 { eprintln!("[vitte] verbose x{}", cli.verbose); }

    match cli.cmd {
        Cmd::Run(a)         => commands::run::exec(a),
        Cmd::Compile(a)     => commands::compile::exec(a),
        Cmd::Bc(a)          => commands::bc::exec(a),
        Cmd::Vm(a)          => commands::vm::exec(a),
        Cmd::Repl(a)        => commands::repl::exec(a),
        Cmd::Fmt(a)         => commands::fmt::exec(a),
        Cmd::Tests(a)       => commands::tests::exec(a),
        Cmd::Bench(a)       => commands::bench::exec(a),
        Cmd::Doc(a)         => commands::doc::exec(a),
        Cmd::Llc(a)         => commands::llc::exec(a),
        Cmd::Pm(a)          => commands::pm::exec(a),
        Cmd::Completions(a) => commands::completions::exec(a),
        Cmd::Man(a)         => commands::man::exec(a),
    }
}