# AutoKey

Auto-presser clavier Windows (Rust 2021, egui 0.29 rendu logiciel, winit 0.30). Un seul exe, sans admin, MAJ auto via GitHub Releases. UI FR/EN/PT. Version : `Cargo.toml` (3.1.0).

## Workflow (budget serré — abonnement Pro)
- Opus = orchestrateur : cadre, lit le minimum, découpe, relit le diff. N'écrit pas le code lui-même sauf micro-fix (< ~10 lignes).
- Implémentation → agent `code-implementer` (Sonnet). Lui passer : fichiers exacts, fonctions visées, critères d'acceptation, contraintes. Pas de contexte superflu.
- Lire par plages (`grep -n` puis `sed -n a,bp`), jamais un gros fichier entier (`src/ui.rs` = 1400 l.).
- Fin de feature → skill `/cloture-session` (MAJ CLAUDE.md + commit conventionnel), puis `/clear`.

## Carte
- `src/` : code, voir `src/CLAUDE.md`.
- `.github/` : CI/release, voir `.github/CLAUDE.md`.
- `build.rs` : manifeste + ressources Windows (icône, version `3.1.0.<AUTOKEY_BUILD>`), Windows seulement.
- `assets/` : `app.ico`, `icon64.rgba` (inclus par `main.rs`).

## Build / test
- Pas de toolchain Rust sur la machine locale → vérif réelle = CI (windows-latest).
- Si cargo dispo : `cargo test` (tourne aussi sous Linux : `NullKeyboard`), `cargo check --features wincheck` (vérifie le code Windows depuis Linux).
- Features : `wincheck` (contrôle code Win hors Windows), `shots` (captures UI dev, `AUTOKEY_SHOTS=dossier`).

## Règles
- ⚠️ Push sur `main` touchant `src/**`, `Cargo.*`, `build.rs`, `assets/**`, workflows = build + release publique + MAJ auto chez les utilisateurs. Ne jamais push sans accord explicite.
- Commits : Conventional Commits (`feat:`, `fix:`, `refactor:`, `docs:`, `chore:`, `ci:`), message en français ok.
- Sécurité non négociable : jamais d'élévation admin, réseau limité aux Releases du dépôt, vérif SHA-256 des MAJ.
- Textes UI : toujours via `i18n` (3 langues), jamais en dur.
