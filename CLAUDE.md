# AutoKey

Auto-presser clavier Windows (Rust 2021, egui 0.29 rendu logiciel, winit 0.30). Un seul exe, sans admin, MAJ auto via GitHub Releases. UI FR/EN/PT. Version : `Cargo.toml` (3.2.0) ; release = tag `v<version>`.

## Workflow (budget serré — abonnement Pro)
- Opus = orchestrateur : cadre, lit le minimum, découpe, relit le diff. N'écrit pas le code lui-même sauf micro-fix (< ~10 lignes).
- Implémentation → agent `code-implementer` (Sonnet). Lui passer : fichiers exacts, fonctions visées, critères d'acceptation, contraintes. Pas de contexte superflu.
- Lire par plages (`grep -n` puis `sed -n a,bp`), jamais un gros fichier entier (`src/ui/widgets.rs` 500 l., `render.rs` 590 l.).
- Skill `/cloture-session` (MAJ CLAUDE.md + commit) : seulement quand l'utilisateur dit vouloir clôturer / arrêter / clear. Jamais spontanément.

## Carte
- `src/` : code, voir `src/CLAUDE.md`.
- `.github/` : CI/release, voir `.github/CLAUDE.md`.
- `build.rs` : manifeste + ressources Windows (icône, version `<version>.<AUTOKEY_BUILD>`), Windows seulement.
- `assets/` : `app.ico`, `icon64.rgba` (inclus par `main.rs`).

## Build / test (toolchain locale installée, Rust épinglé par `rust-toolchain.toml`)
- Avant commit : `cargo fmt --check`, `cargo clippy --locked --all-targets --features shots -- -D warnings`, idem `--target x86_64-pc-windows-gnu` (vérifie le code Windows), `cargo test --locked`. Même chose en CI.
- Exe Windows local : `cargo build --release --target x86_64-pc-windows-gnu`.
- UI : `scripts/ui-shots.sh DOSSIER [REF]` → captures PNG des scènes (`src/shots.rs`) sous Xvfb ; avec REF, comparaison au pixel près (refactor UI = 0 diff).
- Pas de Windows ici : comportement clavier réel non testable, seulement compilé.

## Règles
- ⚠️ Release publique + MAJ auto chez tous les utilisateurs = tag `v*` poussé ou lancement manuel de `build.yml`. Ne jamais tagger/pousser sans accord explicite. Push de branche = CI seule.
- Commits : Conventional Commits (`feat:`, `fix:`, `refactor:`, `docs:`, `chore:`, `ci:`), message en français ok.
- Sécurité non négociable : jamais d'élévation admin, réseau limité aux Releases du dépôt, MAJ vérifiées (SHA-256 + signature minisign).
- Textes UI : toujours via `i18n` (3 langues), jamais en dur.

## En cours
- 3.2.0 (build-14) publiée le 2026-10-13 sans test sur vrai Windows : attendre retour utilisateur (spam, pattern, F6/F7/F8, popup réglages, instance unique). Bug → fix sur `main` + 3.2.1.
- Pistes non faites : icône zone de notification, combos/clics souris dans patterns, profils, signature Authenticode.
