# Makefile (ajoute ces cibles)

.PHONY: man man-install man-clean

# Génère les pages man dans dist/man1 (non compressées)
man:
	cargo run -- man --dir dist --all

# Génère + compresse (si la feature est activée) et installe en ~/.local/share/man/man1
man-install:
	cargo run --features man_gzip -- man --dir ~/.local/share/man --all --gzip
	-@mandb >/dev/null 2>&1 || true
	@echo "✅ man pages installées dans ~/.local/share/man/man1"

man-clean:
	rm -rf dist/man1
