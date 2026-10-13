# src/ — carte des modules

Modules déclarés à plat dans `main.rs`. Flux : `main` → `config::load` → `control::Shared` (état partagé) → `spawn_poller` (lecture clavier) + `update::Updater` + boucle winit → `ui::App` peint via `render`.

| Fichier | Rôle | Points d'entrée |
|---|---|---|
| `main.rs` | boucle winit, fenêtre, cadence (≥33 ms/frame), présentation softbuffer | `Runner` (ApplicationHandler), `main()` |
| `config.rs` | `Settings` (lang, pin, mode, spam, pattern, `press_ms` 0–500 déf. 20, `stop_hotkey` déf. F8 — vidé par `sanitize` si conflit), `Step` Hold/Press/Wait, `Unit`, load/save JSON `%APPDATA%\AutoKey\config3.json`, `sanitize*` (bornes, ≤500 steps, fichier ≤1 Mo), presets `PatternFile`. Sauvegarde atomique (.tmp + rename), JSON illisible copié en `config3.bad.json`. `MIN_INTERVAL_S` = 10 ms | `load`, `save`, `sanitize` |
| `control.rs` | `Shared` : settings+runner+recording sous Mutex ; start/stop/toggle par `Mode`, capture raccourcis (`Target`), `handle(RawKey)`, `check_bindings` (bascule/maintien/arrêt ; un combo ne s'active pas si un sur-ensemble strict est pressé), `conflicts` (refus + flash `hk_conflict`) ; `spawn_poller` (thread, poll 10/15 ms) | `Shared::new`, `start`, `handle` |
| `engine.rs` | `Runner` : 1 thread par `Job` (Spam / Pattern de `CStep`), arrêt interruptible (`Stop`), aucune touche laissée enfoncée. Appui maintenu `press` (spam : ≤ interval/2). Trait `Keyboard` | `Runner::start/stop` |
| `keys.rs` | `KeySpec` (vk + scan), labels localisés, tri/affichage combos, `RawKey` | `combo_label` |
| `platform.rs` | Windows : `SendInput` scancodes, `GetAsyncKeyState`, timer haute résolution, coins arrondis DWM, dialogues `rfd`. Hors Windows : `NullKeyboard` + `AUTOKEY_STDIN_KEYS`. Marqueurs d'injection (`INJ`, 60 ms) pour ignorer nos propres touches. `single_instance` (mutex, attend 8 s l'ancien exe), `release_held` (hook panic + sortie) | `keyboard`, `key_poll` |
| `i18n.rs` | table statique `T: (clé, [fr,en,pt])`, `tr`/`trf` (`{x}`), formats nombres/durées | `tr`, `trf` |
| `ui/mod.rs` | `Act`, `App` (new, `apply`, `update`, `draw`) ; UI → actions `Act` appliquées dans `App::apply` ; réexporte `BG`, `WIDTH`, `height_for`, `setup_fonts` | `App::update` |
| `ui/theme.rs` | palette, `font`/`W`, `setup_fonts`, `WIDTH`/`height_for`, helpers couleur (`lerp_c`, `lighten`, `grad_rect`, `sk`, `kind_color`) | |
| `ui/widgets.rs` | primitives de dessin et widgets à la main (`card`, `text*`, `Ic`/`icon`, boutons, `segmented`, `keycap`, `num_field`, `start_button`, `rhythm`) | |
| `ui/chrome.rs` | `impl App` : barre de titre, popups langue/réglages, modal de confirmation, footer | |
| `ui/spam.rs` | `impl App` : page spam, `hotkey_card`, `hotkey_hint`/`rec_label` | |
| `ui/pattern.rs` | `impl App` : page pattern, `timeline`, `step_card` | |
| `render.rs` | rasteriseur logiciel egui + redessin partiel (hash des shapes, rects sales fusionnés) | `Renderer::render` |
| `update.rs` | MAJ depuis Releases (`AUTOKEY_REPO`/`AUTOKEY_BUILD` compile-time), filtre URL dépôt, ≤64 Mo, SHA-256, signature minisign (`PUBKEY`, cf. `.github/CLAUDE.md`), en-tête MZ, swap atomique + rollback | `Updater::spawn_checker/install` |
| `shots.rs` | captures auto de scènes UI (feature `shots`) | `init`, `tick` |

## Invariants
- Nouvelle chaîne UI → ajouter clé dans `i18n::T` (3 langues) ; clé inconnue = `""` silencieux.
- Nouveau champ `Settings` → valeur dans `impl Default` (écrit à la main) + borne dans `sanitize` + test (compat anciens config3.json).
- Nouvelle action UI → variante `Act` + branche dans `App::apply`.
- Toute touche envoyée passe par `Keyboard` (mockable : `engine::tests::Log`) ; tests dans `engine`, `control`, `config`, `i18n` (clés utilisées ⊂ table : ajouter le fichier à son `include_str!` s'il appelle `tr`), `update`, `render`.
- Nouvelle touche de raccourci/action → passer par `Shared::conflicts`.
- Pas de `unwrap` sur I/O ou réseau ; erreurs MAJ = `&'static str`.
