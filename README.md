# Chartographer

Cockpit de bureau pour les statistiques de mods Minecraft publiés sur Modrinth et CurseForge.
Agrège les deux plateformes, croise les projets, et rend le tout sur une page de vision unique.

## Ce que ça montre

- Téléchargements par jour, par mod, par plateforme, avec zoom temporel
- Répartition géographique des téléchargements sur une carte mondiale
- Part Modrinth contre part CurseForge, mod par mod
- Concentration des téléchargements par version de jeu et par loader
- Revenus Modrinth, journaliers et cumulés
- Fil d'évènements et notifications

Les projets sont découverts automatiquement. Aucun identifiant à saisir à la main.

## Sources

**Modrinth** — API authentifiée. Séries temporelles complètes côté serveur : téléchargements, vues, revenus, pays.

**CurseForge** — le token d'upload auteur ne donne pas accès aux statistiques. Les totaux sont lus via CFWidget, qui ne conserve pas d'historique. Chartographer prend un snapshot quotidien et reconstruit la courbe localement. La courbe CurseForge est donc vide les premiers jours après installation.

## Installation

Des installeurs `.deb`, `.rpm` et `.exe` sont publiés dans les [Releases](../../releases).

## Configuration

Au premier lancement, l'écran de réglages demande :

| Clé | Où la trouver |
|---|---|
| `MODRINTH_TOKEN` | Modrinth → Settings → PATs |
| `CURSEFORGE_UPLOAD_TOKEN` | CurseForge → My Account → API Token |
| `CURSEFORGE_USERNAME` | ton pseudo auteur CurseForge |

Ces valeurs sont écrites dans un `.env` du dossier de configuration applicatif. Elles ne quittent jamais le processus Rust et ne sont jamais transmises à l'interface.

## Développement

```bash
npm install
npm run tauri dev
```

Prérequis : Node 24, Rust 1.95, et les dépendances système Tauri v2 pour ta plateforme.

```bash
npm run check          # svelte-check
npm test               # vitest
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

## Architecture

Le réseau et la persistance vivent entièrement côté Rust. La webview ne voit ni les tokens ni les réponses brutes des API.

```
src-tauri/src/
  providers/    clients Modrinth et CFWidget
  store/        SQLite, migrations, accès typés
  matching.rs   appariement inter-plateformes
  sync.rs       orchestration
  commands.rs   surface exposée au front
src/            interface Svelte 5 + ECharts
```

Le design complet est dans [docs/superpowers/specs/2026-08-11-chartographer-design.md](docs/superpowers/specs/2026-08-11-chartographer-design.md).

## Licence

MIT
