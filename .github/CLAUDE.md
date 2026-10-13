# .github/ — CI/release

- `workflows/ci.yml` : chaque push/PR (hors `*.md`), ubuntu. `fmt --check`, clippy `-D warnings` Linux (+feature `shots`) et Windows (`x86_64-pc-windows-gnu`, mingw), `cargo test`, `cargo audit` (bloquant). Ne publie rien.
- `workflows/build.yml` : release, **uniquement** sur tag `v*` (doit égaler `version` de Cargo.toml) ou lancement manuel. Jobs : `build` (windows, tests + exe) + `audit` → `release` (ubuntu : sha256, signature minisign, attestation, `gh release create build-N --latest`).
- Numéro N = max(dernier tag `build-N` + 1, run_number) → toujours croissant ; c'est `AUTOKEY_BUILD`, comparé par `update::parse_release`. Ne pas changer le format `build-N`.
- Assets figés (consts `update.rs`) : `AutoKey.exe`, `.sha256`, `.minisig`.
- Signature : secret `MINISIGN_KEY` (clé privée minisign sans mot de passe) ; clé publique en dur dans `update.rs` ET `build.yml` (`MINISIGN_PUBKEY`) — changer les deux ensemble. Secret absent → release échoue (voulu).
- Actions épinglées par SHA (commentaire = version). Toolchain figée par `rust-toolchain.toml`.
- `--locked` partout : nouvelle dépendance ⇒ `Cargo.lock` commité.
