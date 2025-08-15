```
├── .cargo
├── .github
│   ├── ISSUE_TEMPLATE
│   │   ├── bug.md
│   │   ├── feature.md
│   ├── workflows
│   │   ├── ci.yml
│   │   ├── docs.yml
│   │   ├── perf.yml
│   │   ├── pr-fast.yml
│   │   ├── release.yml
├── assets
├── benchmarks
│   ├── macro
│   ├── micro
├── cli-todo
│   ├── main.vitte
├── crates
│   ├── vitte-cli
│   │   ├── src
│   │   │   ├── lib.rs
│   │   │   ├── main.rs
│   │   ├── Cargo.toml
│   ├── vitte-compiler
│   │   ├── src
│   │   │   ├── lib.rs
│   │   ├── Cargo.toml
│   ├── vitte-core
│   │   ├── src
│   │   │   ├── bin
│   │   │   │   ├── Cargo.toml
│   │   │   │   ├── vitte-asm.rs
│   │   │   │   ├── vitte-disasm.rs
│   │   │   │   ├── vitte-link.rs
│   │   │   ├── bytecode
│   │   │   │   ├── chunk.rs
│   │   │   │   ├── disasm.rs
│   │   │   │   ├── format.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── ops.rs
│   │   │   ├── compiler
│   │   │   │   ├── config.rs
│   │   │   │   ├── driver.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── output.rs
│   │   │   ├── runtime
│   │   │   │   ├── eval.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── parser.rs
│   │   │   │   ├── pretty.rs
│   │   │   │   ├── tokenizer.rs
│   │   │   ├── vitte-vm
│   │   │   │   ├── interpreter.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── stack.rs
│   │   │   ├── asm.rs
│   │   │   ├── Cargo.toml
│   │   │   ├── lib.rs
│   │   │   ├── loader.rs
│   │   │   ├── util.rs
│   │   ├── tests
│   │   │   ├── integration.rs
│   │   │   ├── README.md
│   │   ├── Cargo.toml
│   ├── vitte-runtime
│   │   ├── src
│   │   │   ├── lib.rs
│   │   ├── Cargo.toml
│   ├── vitte-stdlib
│   │   ├── src
│   │   │   ├── lib.rs
│   │   ├── Cargo.toml
│   ├── vitte-tools
│   │   ├── src
│   │   │   ├── bin
│   │   │   │   ├── vitte-asm.rs
│   │   │   │   ├── vitte-disasm.rs
│   │   │   │   ├── vitte-fmt.rs
│   │   │   │   ├── vitte-link.rs
│   │   │   │   ├── vitte-repl.rs
│   │   │   ├── lib.rs
│   │   ├── Cargo.toml
│   ├── vitte-vm
│   │   ├── src
│   │   │   ├── lib.rs
│   │   ├── Cargo.toml
├── desktop
│   ├── gtk_real.c
│   ├── gtk_stub.c
│   ├── main.vitte
│   ├── Makefile
│   ├── qt_real.cpp
│   ├── qt_stub.cpp
│   ├── README.md
├── docs
│   ├── arborescence.md
├── editor-plugins
│   ├── emacs
│   ├── vim
│   ├── vscode
├── embedded-blink
│   ├── main.vitte
├── examples
│   ├── hello
│   │   ├── src
│   │   │   ├── main.vit
│   │   ├── vitte.toml
│   ├── hello-vitte
│   │   ├── main.vitte
│   ├── kernel
│   │   ├── armv7em
│   │   │   ├── kmain.vitte
│   │   │   ├── linker.ld
│   │   │   ├── start.S
│   │   ├── x86_64
│   │   │   ├── kmain.vitte
│   │   │   ├── linker.ld
│   │   │   ├── start.S
│   │   ├── README.md
│   ├── wasm-add
│   │   ├── main.vitte
│   ├── web-echo
│   │   ├── main.vitte
│   │   ├── middleware.vitte
│   │   ├── README.md
│   ├── worker-jobs
├── modules
│   ├── cache.vitte
│   ├── channel.vitte
│   ├── checksum.vitte
│   ├── cli.vitte
│   ├── config.vitte
│   ├── csv.vitte
│   ├── eventbus.vitte
│   ├── exports.vitte
│   ├── feature_flags.vitte
│   ├── fs_atomic.vitte
│   ├── graph.vitte
│   ├── http_client.vitte
│   ├── idgen.vitte
│   ├── ini.vitte
│   ├── kvstore.vitte
│   ├── log.vitte
│   ├── main.vitte
│   ├── mathx.vitte
│   ├── metrics.vitte
│   ├── migrate.vitte
│   ├── pagination.vitte
│   ├── plugin.vitte
│   ├── pool.vitte
│   ├── prioq.vitte
│   ├── random.vitte
│   ├── rate_limiter.vitte
│   ├── README.md
│   ├── result_ext.vitte
│   ├── retry.vitte
│   ├── rle.vitte
│   ├── scheduler.vitte
│   ├── stringx.vitte
│   ├── supervisor.vitte
│   ├── taskpool.vitte
│   ├── tracing.vitte
│   ├── util.vitte
│   ├── uuid.vitte
│   ├── validate.vitte
│   ├── yaml_lite.vitte
├── rfcs
│   ├── rfcs
│   │   ├── NNNN-ownership-model.md
│   ├── 0000-template.md
├── scripts
│   ├── build_all.sh
│   ├── ci_check.sh
│   ├── clean_all.sh
│   ├── compress_bytecode.py
│   ├── decompress_bytecode.py
│   ├── fmt.sh
│   ├── gen_bytecode.sh
│   ├── gen_docs.sh
│   ├── gen_licenses.sh
│   ├── lint.sh
│   ├── release.sh
│   ├── release_package.sh
│   ├── test.sh
│   ├── test_all.sh
├── security
├── tests
│   ├── compiler
│   ├── fixtures
│   ├── integration
│   ├── performance
│   ├── unit
│   ├── vm
├── tools
│   ├── vitc
│   │   ├── src
│   │   │   ├── main.rs
│   │   ├── Cargo.toml
│   ├── vitcc
│   │   ├── src
│   │   │   ├── main.rs
│   │   ├── Cargo.toml
│   ├── vitpm
│   │   ├── src
│   │   │   ├── main.rs
│   │   ├── Cargo.toml
│   ├── vitte-bench
│   │   ├── src
│   │   │   ├── main.rs
│   │   ├── Cargo.toml
│   ├── vitte-doc
│   │   ├── src
│   │   │   ├── main.rs
│   │   ├── Cargo.toml
│   ├── vitte-fmt
│   │   ├── src
│   │   │   ├── main.rs
│   │   ├── Cargo.toml
│   ├── vitte-profile
│   │   ├── src
│   │   │   ├── main.rs
│   │   ├── Cargo.toml
│   ├── vitx
│   │   ├── src
│   │   │   ├── main.rs
│   │   ├── Cargo.toml
│   ├── vitxx
│   │   ├── src
│   │   │   ├── main.rs
│   │   ├── Cargo.toml
├── .gitattributes
├── .gitignore
├── arborescence.md
├── build.rs
├── Cargo.toml
├── clippy.toml
├── deny.toml
├── LICENSE
├── main.vitte
├── Makefile
├── README.md
├── rustfmt.toml
├── rust-toolchain.toml
```
