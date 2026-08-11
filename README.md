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

**CurseForge** — aucune authentification. Les totaux sont lus via CFWidget, qui ne conserve pas d'historique. Chartographer prend un snapshot quotidien et reconstruit la courbe localement. La courbe CurseForge est donc vide les premiers jours après installation.

## Installation

Des installeurs `.deb`, `.rpm` et `.exe` sont publiés dans les [Releases](../../releases).

## Configuration

Une seule chose à faire, une seule fois : coller un token Modrinth.

L'écran d'accueil ouvre directement [modrinth.com/settings/pats](https://modrinth.com/settings/pats) pour toi. Crée un token en cochant ces six autorisations, toutes en lecture seule :

`Read user data` · `Read notifications` · `Read payouts` · `Access analytics` · `Read projects` · `Read versions`

Colle-le, c'est terminé. Le token est validé auprès de Modrinth avant d'être enregistré dans `session.json`, dans le dossier de données applicatif. Il ne quitte jamais le processus Rust et n'est jamais transmis à l'interface. Se déconnecter supprime le fichier.

**CurseForge ne demande rien.** Le pseudo auteur est retrouvé automatiquement en interrogeant CFWidget avec tes slugs Modrinth. Un champ de réglage permet de le corriger dans le cas improbable où la détection échoue.

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
  config.rs     session, réglages
  matching.rs   appariement inter-plateformes
  sync.rs       orchestration
  commands.rs   surface exposée au front
src/            interface Svelte 5 + ECharts
```

L'interface suit le thème clair ou sombre du système, avec un bouton pour forcer l'un ou l'autre.

Le design complet est dans [docs/superpowers/specs/2026-08-11-chartographer-design.md](docs/superpowers/specs/2026-08-11-chartographer-design.md).

## Licence

MIT
