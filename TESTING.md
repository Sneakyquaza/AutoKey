# Tester AutoKey

## Activer les hooks Git

```sh
git config core.hooksPath .githooks
```

| Hook | Quand | Contenu | Durée |
|---|---|---|---|
| `pre-commit` | chaque `git commit` (ignoré si seuls des `.md` changent) | `cargo fmt --check`, clippy Linux et Windows (`-D warnings`), `cargo test`, `cargo audit` si installé | ~30 s |
| `pre-push` | chaque `git push` (donc aussi avant un tag de release) | `scripts/test-all.sh` complet | ~1-2 min |

`AUTOKEY_FAST=1 git push` saute les captures UI. Contournement exceptionnel : `--no-verify`.

La CI (`.github/workflows/ci.yml`) refait fmt, clippy Linux/Windows, tests et `cargo audit` (bloquant) à chaque push. La release (`build.yml`, tag `v*`) rejoue les tests et l'audit avant de publier.

## Tout lancer en une commande

```sh
scripts/test-all.sh
```

Étapes : fmt, clippy Linux, clippy Windows, tests, `cargo audit` (avertissement s'il est absent), build release Windows (`x86_64-pc-windows-gnu`), puis captures UI dans `target/shots-check` avec vérification du nombre de PNG et qu'aucun n'est vide (sautées avec un message si Xvfb est absent). Code retour non nul au premier échec.

Prérequis : toolchain épinglée (`rust-toolchain.toml`) avec la cible `x86_64-pc-windows-gnu` et mingw, `cargo install cargo-audit --locked`, `apt install xvfb xauth fonts-inter`.

## Captures UI

```sh
scripts/ui-shots.sh DOSSIER            # captures .ppm + .png de toutes les scènes (src/shots.rs)
scripts/ui-shots.sh DOSSIER REFERENCE  # idem, puis comparaison au pixel près avec REFERENCE
```

Pour valider un refactor d'interface sans changement visuel : générer une référence avant (`scripts/ui-shots.sh /tmp/ref`), modifier le code, puis `scripts/ui-shots.sh /tmp/new /tmp/ref` doit afficher `OK` partout. Un changement voulu se vérifie en ouvrant les PNG (français, anglais et portugais, le portugais étant le plus long). Les scènes se déclarent dans `SCENES` et `setup` de `src/shots.rs`.

## Checklist manuelle sur Windows avant release

Le comportement clavier réel n'est pas testable hors Windows (le code est seulement compilé). Tester l'exe de `build.yml` ou `cargo build --release --target x86_64-pc-windows-gnu`, avec un éditeur de texte ou un testeur de touches ouvert.

- [ ] **Spam** : choisir une touche, régler l'intervalle (ms / s / min), démarrer et arrêter par le bouton et par le raccourci (F6 par défaut) ; la touche est bien reçue au bon rythme.
- [ ] **Raccourcis** : mode bascule et mode maintien pour spam et pattern ; changer un raccourci ; un conflit entre raccourcis est refusé avec un message ; F8 arrête tout.
- [ ] **Limite d'arrêt automatique** : par nombre de pressions, par durée ; la tâche s'arrête seule, le raccourci la relance ensuite.
- [ ] **Jitter** : avec 30 %, les intervalles varient visiblement ; à 0 % ils sont réguliers.
- [ ] **Bip** : un bip au démarrage et à l'arrêt, y compris fenêtre cachée.
- [ ] **Pattern** : ajouter des étapes Maintenir / Appuyer / Attendre, réordonner, dupliquer, supprimer ; boucles finies et infinies ; l'étape active est suivie dans la frise.
- [ ] **Macro** : Enregistrer, taper une séquence (y compris touche maintenue et pause), arrêter par re-clic puis par F8 ; les touches de raccourci ne sont pas enregistrées ; la séquence rejouée est fidèle ; remplacement d'un pattern existant après confirmation.
- [ ] **Profils** : créer, charger, renommer, écraser, supprimer ; les raccourcis globaux ne changent pas au chargement ; charger un profil pendant une tâche l'arrête.
- [ ] **Liaison à la fenêtre** : démarrer dans une fenêtre, passer dans une autre : les touches s'interrompent, relâchées, et reprennent au retour ; fermer la fenêtre cible arrête la tâche.
- [ ] **Zone de notification** : icône présente, menu clic droit (Afficher / Quitter), fermer et réduire cachent la fenêtre, l'icône revient après un redémarrage de l'explorateur.
- [ ] **Démarrage caché** : « Démarrer caché » lance l'exe sans fenêtre visible ; lancer une 2e instance ré-affiche la première.
- [ ] **Démarrage avec Windows** : activer, se déconnecter / reconnecter, AutoKey démarre ; désactiver retire l'entrée.
- [ ] **Position de la fenêtre** : déplacer sur l'écran 2, fermer (et Quitter via l'icône), relancer → rouvre au même endroit sur l'écran 2 ; débrancher l'écran 2 puis relancer → fenêtre visible, centrée sur l'écran principal ; démarrage caché puis Afficher → bonne position ; réduite dans la barre des tâches, ne pas perdre la position.
- [ ] **Mises à jour** : « Vérifier maintenant » (à jour, puis avec une version plus ancienne installée : le bouton « Mettre à jour » apparaît) ; mise à jour réelle jusqu'au redémarrage sur la nouvelle version ; désactiver la vérification automatique est pris en compte.
- [ ] **Import / export / réinitialisation** : exporter, modifier, importer (réglages restaurés) ; un fichier invalide ou en conflit est refusé ; réinitialiser garde profils et langue.
- [ ] **Vue compacte** : bascule dans les deux sens, redimensionnement correct, démarrer / arrêter, changement de mode, panneau Réglages ouvert en vue compacte.
- [ ] **Info-bulles « i »** : présentes sur les pages spam, pattern, profils et Réglages, lisibles et sans chevauchement en FR / EN / PT, et jamais coupées au bord de la fenêtre.
- [ ] **Aucune touche bloquée** : pendant une tâche (spam et maintien long), quitter par la croix, par la zone de notification, par fin de session ou en tuant le processus : aucune touche ne reste enfoncée.
- [ ] **Instance unique** : lancer l'exe deux fois ; la seconde attend / remplace proprement, jamais deux claviers simulés en même temps.
- [ ] **Langues** : FR / EN / PT basculent partout (barre de titre, popups, Réglages) sans texte tronqué.
