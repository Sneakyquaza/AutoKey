---
name: cloture-session
description: Clôture une session AutoKey (MAJ des CLAUDE.md imbriqués + commit Conventional Commits). À utiliser UNIQUEMENT quand l'utilisateur dit explicitement vouloir clôturer / arrêter là / faire un clear / finir la session. Jamais de sa propre initiative, même en fin de feature.
---

# Clôture de session

But : qu'une session neuve, après `/clear`, reprenne avec le minimum de contexte exact. Rester économe : pas de relecture complète du code.

## 1. Constater ce qui a changé
- `git status --short` et `git diff --stat HEAD` (+ `git log --oneline -5` si commits pendant la session).
- Lire uniquement les hunks utiles (`git diff HEAD -- <fichier>`), pas les fichiers entiers.
- Ajouter ce que la conversation a appris et que le diff ne montre pas (décisions, pièges, travail en cours).

## 2. Mettre à jour les CLAUDE.md
Fichiers : `CLAUDE.md` (global, workflow, règles), `src/CLAUDE.md` (carte modules + invariants), `.github/CLAUDE.md` (CI/release). Si un nouveau dossier de code naît avec des règles propres, créer son `CLAUDE.md` et le référencer dans la carte du parent.

Pour chaque fichier concerné :
- **Corriger / supprimer** toute info devenue fausse (fonctions renommées, numéros de ligne décalés, version, dépendances). Vérifier avec `grep -n` avant de garder un nom ou une ligne.
- **Ajouter** seulement ce qui sert à reprendre le travail : nouveau module/point d'entrée, nouvel invariant, piège découvert, décision d'archi. Pas d'historique, pas de changelog (git le garde), pas ce qui se lit trivialement dans le code.
- **Factoriser** : une info à un seul endroit, au niveau le plus local qui la concerne ; le parent pointe vers l'enfant.
- **Plafonds** : `CLAUDE.md` racine ≤ 40 lignes, chaque imbriqué ≤ 50 lignes. Au-delà : condenser ou déplacer vers un CLAUDE.md plus local.
- Travail inachevé → section `## En cours` dans le CLAUDE.md racine (≤ 5 lignes : quoi, où, prochaine étape). La vider quand c'est fini.

## 3. Commit
- `git add -A` puis vérifier `git status` (aucun binaire, `target/`, secret).
- Un commit par sujet logique si le diff mélange plusieurs choses, sinon un seul.
- Format Conventional Commits : `type(scope): description` — types `feat`, `fix`, `refactor`, `perf`, `test`, `docs`, `ci`, `chore` ; scope = module (`ui`, `engine`, `update`…). Corps optionnel : le pourquoi. Màj seule des CLAUDE.md → `docs(claude): …`.
- Ne pas push (un push sur `main` publie une release). Le proposer à l'utilisateur.

## 4. Rapport (3 lignes max)
Commits créés, CLAUDE.md modifiés, état `## En cours`. Puis : « Tu peux faire `/clear`. »
