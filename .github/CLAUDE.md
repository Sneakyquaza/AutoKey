# .github/ — CI/release

`workflows/build.yml` (windows-latest), déclenché par push `main`/`master` sur `src/**`, `assets/**`, `Cargo.*`, `build.rs`, workflows (hors `*.md`) ou manuellement.
Étapes : cargo-audit (non bloquant) → `cargo test --release --locked` → build → `AutoKey.exe` + `.sha256` → attestation provenance → `gh release create build-<run_number> --latest`.
- `AUTOKEY_BUILD` = run_number = numéro comparé par l'updater (tag `build-N`) : ne pas changer le format du tag sans adapter `update::parse_release`.
- Noms d'assets `AutoKey.exe` / `AutoKey.exe.sha256` figés (consts dans `update.rs`).
- `--locked` : toute nouvelle dépendance exige `Cargo.lock` à jour.
