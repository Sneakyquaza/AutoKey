# AutoKey

Auto-presser clavier pour Windows : spam d'une touche, patterns de touches en boucle, raccourcis globaux (bascule ou maintien), interface FR / EN / PT.

## Télécharger

Onglet **Releases** → dernière version → `AutoKey.exe`. Aucun installateur, aucune dépendance, aucun droit administrateur.
L'appli se met ensuite à jour toute seule (bouton **Mettre à jour** dans la barre du haut quand une nouvelle version est publiée).

## Sécurité

- S'exécute avec les droits normaux de l'utilisateur (jamais d'élévation admin) ; réglages dans `%APPDATA%\AutoKey`.
- Seule connexion réseau : vérification des mises à jour sur les Releases GitHub de ce dépôt (HTTPS uniquement).
- Mise à jour acceptée seulement si : le fichier vient de ce dépôt, son empreinte SHA-256 correspond à celle publiée, il porte une signature minisign valide de la clé du projet (intégrée à l'appli), et c'est bien un exécutable Windows. Sinon elle est refusée.
- Chaque exe est compilé par GitHub Actions à partir de ce code ; tests, lints et audit des dépendances (RustSec) doivent passer avant toute publication.

### Vérifier un exe téléchargé

Empreinte (PowerShell) — doit être identique au contenu de `AutoKey.exe.sha256` :

    Get-FileHash .\AutoKey.exe -Algorithm SHA256

Signature ([minisign](https://jedisct1.github.io/minisign/)) — avec `AutoKey.exe.minisig` à côté de l'exe :

    minisign -Vm AutoKey.exe -P RWRpMB1oOVO8oAhJTqnFniOXTkaykl7/VHz7aPE2WQodN+jhG6DtcX6V

Provenance (dépôt public, avec GitHub CLI) — prouve que l'exe a été compilé par ce dépôt :

    gh attestation verify .\AutoKey.exe -R <propriétaire>/<dépôt>

## Avertissement Windows / antivirus

L'exe n'est pas encore signé : SmartScreen affiche donc « éditeur inconnu » au premier lancement
(*Informations complémentaires → Exécuter quand même*). Un logiciel qui lit le clavier et simule des touches
peut aussi être signalé à tort par certains antivirus. Faux positif → https://www.microsoft.com/wdsi/filesubmission

## Licence

MIT — © 2026 sneakyquaza
