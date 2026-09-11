# NVDA-RUST-UIA-STANDALONE

Lecteur d’écran Windows expérimental écrit en Rust, basé directement sur Microsoft UI Automation (UIA). Le projet est indépendant du code de NVDA : NVDA officiel est conservé séparément comme référence de comparaison en lecture seule.

## Base Windows validée

La base actuellement validée sait :

- initialiser COM en MTA et créer le client UI Automation Microsoft ;
- écouter globalement les changements de focus UIA ;
- récupérer un focus manquant via `GetFocusedElement()` avec déduplication ;
- écouter `TextChanged` (`UIA 20015`) et `TextSelectionChanged` (`UIA 20014`) ;
- écouter les propriétés `ValueValue`, `SelectionItemIsSelected` et `ToggleToggleState` ;
- compléter les fournisseurs UIA incomplets par échantillonnage `ValuePattern` / `TogglePattern` ;
- suivre l’état par contrôle avec stockage borné ;
- extraire PID, framework, classe, rôle, nom et `AutomationId` ;
- récupérer caret et sélection via `IUIAutomationTextPattern2` quand le fournisseur le permet ;
- protéger les champs mot de passe avant toute lecture de valeur ou de sélection ;
- envoyer les annonces vers un worker TTS asynchrone Windows `SpeechSynthesizer` + `MediaPlayer` ;
- annoncer le focus et l’état des cases à cocher sans bloquer les callbacks UIA ;
- vider proprement la file TTS à l’arrêt et publier des compteurs vérifiables ;
- exécuter un parcours fonctionnel 100 % clavier sur une fixture WinForms et sur le Bloc-notes natif ;
- vérifier automatiquement qu’aucune interaction souris synthétique n’est utilisée dans ce parcours ;
- conserver une référence officielle `nvaccess/nvda` épinglée pour les comparaisons d’architecture et de comportement.

Le parcours intégré validé couvre notamment l’édition, Tab / Shift+Tab, activation d’une case à cocher, modification d’une liste déroulante, activation d’un bouton, déplacement de caret et sélection au clavier.

## Confidentialité

Les champs protégés sont traités avant toute extraction de contenu :

- aucun `ValuePattern` n’est interrogé pour un champ mot de passe ;
- TextPattern2 produit uniquement une preuve expurgée ;
- le contenu du secret de test ne doit apparaître ni dans les logs, ni dans les preuves, ni dans la parole ;
- le focus est annoncé avec la phrase générique `password field` ;
- le scénario CI de confidentialité recherche explicitement toute fuite du secret.

## Architecture actuelle

```text
src/
  lib.rs                   noyau portable et modèle d’événements
  main.rs                  point d’entrée multi-plateforme
  windows_runtime_v2.rs    abonnement UIA, focus, événements et fallbacks d’état
  windows_speech.rs        worker de synthèse vocale asynchrone
  windows_textpattern2.rs  caret, sélection et protection des champs sensibles

tests/windows/
  BlindUserFixture.ps1          fixture WinForms accessible
  RunBlindUserKeyboardE2E.ps1   parcours utilisateur aveugle clavier-only
  AssertSpeechEvidence.ps1      validation du pipeline vocal
  AssertTextPattern2Evidence.ps1 validation caret/sélection
  PasswordPrivacyFixture.ps1    champ protégé de test
  RunPasswordPrivacyE2E.ps1     validation anti-fuite

reference/
  nvda-baseline.json       commit officiel NVDA utilisé comme référence
```

## Référence NVDA officielle

`reference/nvda-baseline.json` pointe vers `https://github.com/nvaccess/nvda.git` en mode référence uniquement. La CI récupère le SHA épinglé et vérifie qu’il correspond exactement avant les validations comparatives.

La référence NVDA n’est pas utilisée comme base de code du nouveau lecteur d’écran.

## Compilation Windows

Prérequis : Windows x64, Rust stable et cible MSVC.

```powershell
cargo build --release --locked --target x86_64-pc-windows-msvc
```

Exécutable :

```text
target\x86_64-pc-windows-msvc\release\nvda-rust-uia-standalone.exe
```

## Exécution

Surveiller UI Automation pendant 15 secondes :

```powershell
cargo run --release -- 15
```

Le runtime écrit des marqueurs déterministes pour les événements UIA, TextPattern2, la parole et les fallbacks afin de permettre une validation reproductible.

## Validation locale

```powershell
cargo fmt --all -- --check
cargo check --all-targets --locked
cargo test --all-targets --locked
cargo clippy --all-targets --locked --no-deps -- -D warnings
cargo build --release --locked --target x86_64-pc-windows-msvc
```

## Validation fonctionnelle Windows

Le workflow principal exécute un parcours externe comme un utilisateur clavier :

- saisie dans une zone d’édition ;
- Tab et Shift+Tab entre contrôles ;
- Space sur une case à cocher ;
- flèche dans une liste déroulante ;
- Enter sur un bouton ;
- édition réelle dans Bloc-notes ;
- sélection par Shift+flèche ;
- fermeture des applications au clavier ;
- vérification des événements UIA et des sorties TTS correspondantes.

Les assertions distinguent la génération du flux vocal de la sortie physique sur haut-parleurs : la CI valide le pipeline jusqu’à `MediaPlayer.Play()`, mais ne prétend pas mesurer le son réellement entendu par une carte audio.

## État encore non terminé

Ce dépôt n’est pas encore un lecteur d’écran complet. Les principaux chantiers restants sont notamment :

1. comportement vocal de sélection/caret proche d’un lecteur d’écran mature ;
2. politique d’interruption et de coalescence de la parole pendant la navigation rapide ;
3. navigation web/document et navigation structurée ;
4. couverture réelle Edge/Chromium, applications Windows, Office et terminaux ;
5. couche de commandes et gestes globale ;
6. braille ;
7. configuration utilisateur, profils, localisation et diagnostics ;
8. packaging/installateur et démarrage automatique ;
9. comparaison runtime automatisée NVDA officiel vs moteur Rust sur les mêmes scénarios.

L’objectif de validation reste de mesurer les comportements réels et de séparer explicitement ce qui est **PASS** de ce qui n’a pas encore été testé.
