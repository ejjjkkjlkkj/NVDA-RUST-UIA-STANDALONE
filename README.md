# NVDA-RUST-UIA-STANDALONE

Prototype de moteur de lecteur d’écran Windows en Rust, basé directement sur Microsoft UI Automation (UIA), avec un noyau portable testable sur Windows, Linux et macOS.

## État actuel

Le prototype sait actuellement :

- initialiser COM en MTA avec nettoyage automatique RAII ;
- activer `CUIAutomation8` et `IUIAutomation6` quand disponibles ;
- basculer automatiquement vers `IUIAutomation` en mode compatibilité ;
- écouter les changements de focus UIA ;
- écouter `TextChanged` (`UIA 20015`) ;
- écouter `TextSelectionChanged` (`UIA 20014`) ;
- extraire PID, framework, classe, rôle, nom et AutomationId ;
- normaliser ces données dans un modèle d’événement portable ;
- tester le noyau indépendamment de Windows ;
- compiler le runtime natif Windows x64 avec Rust stable MSVC ;
- produire automatiquement un exécutable Release et son SHA-256.

## Architecture

```text
src/
  lib.rs          noyau portable, modèle d’événement et tests
  main.rs         lanceur multi-plateforme
  windows_app.rs  backend Microsoft UI Automation / COM
```

Linux et macOS compilent le noyau portable et le lanceur non-Windows. Le backend UIA et les dépendances `windows-rs` sont activés uniquement pour `cfg(windows)`.

## Prérequis Windows

- Windows x64
- Rust stable
- cible `x86_64-pc-windows-msvc`

`rust-toolchain.toml` suit automatiquement le canal Rust stable et installe `rustfmt`, `clippy` et la cible Windows x64 MSVC.

## Compilation Windows

```powershell
cargo build --release --locked --target x86_64-pc-windows-msvc
```

Exécutable :

```text
target\x86_64-pc-windows-msvc\release\nvda-rust-uia-standalone.exe
```

## Exécution UIA

Surveiller UI Automation pendant 15 secondes :

```powershell
cargo run --release -- 15
```

Un argument `0` permet un smoke run non interactif très court.

## Validation locale

```powershell
cargo fmt --all -- --check
cargo check --all-targets --locked
cargo test --all-targets --locked
cargo clippy --all-targets --locked --no-deps -- -D warnings
cargo build --release --locked --target x86_64-pc-windows-msvc
```

## GitHub Actions

La CI principale valide :

- Ubuntu latest ;
- macOS latest ;
- Windows latest ;
- compilation croisée du backend Windows depuis Ubuntu et macOS ;
- build Release final sur Windows Server 2025 / Visual Studio 2026 ;
- `cargo fmt`, `cargo check`, tests et `clippy` ;
- publication de l’exécutable x64 et de son SHA-256 comme artefacts.

Des workflows supplémentaires assurent :

- audit RustSec automatique des dépendances ;
- vérification hebdomadaire avec Rust beta et nightly sur Windows/Linux/macOS ;
- release GitHub automatique pour les tags `v*` ;
- Dependabot pour Cargo et GitHub Actions.

Les workflows utilisent les générations actuelles `actions/checkout@v7` et `actions/upload-artifact@v7`, tandis que Rust suit le canal stable.

## Synchroniser le poste Windows

```powershell
cd "C:\NVDA-RUST-UIA-STANDALONE"
git pull origin main
rustup update stable
cargo test --all-targets --locked
cargo build --release --locked --target x86_64-pc-windows-msvc
```

## Direction du projet

Étapes fonctionnelles prévues :

1. cache UIA et réduction des appels COM inter-processus ;
2. file d’événements asynchrone entre UIA et traitement ;
3. récupération texte, caret et sélection via TextPattern/TextPattern2 ;
4. navigation objet et arbre UIA ;
5. couche speech ;
6. braille ;
7. gestion clavier/gestes ;
8. configuration, logs et diagnostics ;
9. tests runtime Windows reproductibles.
