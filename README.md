# NVDA-RUST-UIA-STANDALONE

Prototype Windows x64 de moteur d’accessibilité en Rust basé directement sur Microsoft UI Automation (UIA).

## État actuel

Le prototype sait actuellement :

- initialiser COM en mode multithread ;
- activer `CUIAutomation8` et `IUIAutomation6` quand disponibles ;
- basculer automatiquement vers `IUIAutomation` en mode compatibilité ;
- écouter les changements de focus UIA ;
- écouter `TextChanged` (`UIA 20015`) ;
- écouter `TextSelectionChanged` (`UIA 20014`) ;
- exposer le PID, le framework, la classe, le rôle, le nom et l’AutomationId des éléments ;
- compiler en Rust stable x64 MSVC ;
- être validé automatiquement par GitHub Actions.

## Prérequis

- Windows x64
- Rust stable
- cible `x86_64-pc-windows-msvc`

Le fichier `rust-toolchain.toml` configure automatiquement la dernière version stable de Rust, `rustfmt`, `clippy` et la cible MSVC x64.

## Compilation

```powershell
cargo build --release --locked
```

Exécutable :

```text
target\release\nvda-rust-uia-standalone.exe
```

## Exécution

Surveiller UI Automation pendant 15 secondes :

```powershell
cargo run --release -- 15
```

Pendant l’exécution, déplacer le focus, saisir du texte et modifier la sélection dans des applications Windows.

## Validation locale

```powershell
cargo check --all-targets --locked
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo build --release --locked
```

## CI GitHub

La CI utilise :

- Windows Server 2025 avec Visual Studio 2026 ;
- la dernière version stable de Rust ;
- `actions/checkout@v7` ;
- cache Cargo ;
- compilation avec warnings interdits ;
- publication de l’exécutable Release comme artefact GitHub Actions.

Dependabot vérifie chaque semaine les dépendances Cargo et GitHub Actions.

## Direction du projet

Objectif : faire évoluer ce prototype vers un moteur de lecteur d’écran Windows moderne en Rust, avec une architecture modulaire pour UIA, événements, navigation, texte, synthèse vocale, braille et intégration système.
