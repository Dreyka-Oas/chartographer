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

Des installeurs `.deb`, `.rpm`, `.AppImage` et `.exe` sont publiés dans les [Releases](../../releases).

## Mises à jour

Une fois installée, l'application se tient à jour toute seule. Au démarrage, elle demande à GitHub s'il existe une version plus récente ; si oui, une pastille paraît dans la barre du haut et la section **Réglages → Mises à jour** propose de l'installer. Rien ne se télécharge ni ne s'installe sans ce clic, et la recherche au démarrage se coupe depuis la même section.

Chaque archive est signée avec la clé du projet au moment de la publication, et vérifiée par l'application avant d'être installée : un fichier qui n'a pas été signé avec cette clé est refusé, quelle que soit sa provenance. La clé publique est inscrite dans `src-tauri/tauri.conf.json` ; la clé privée ne vit que dans l'environnement GitHub `release`, dont la règle de déploiement n'accepte que les tags `v*`.

Le `.deb` et le `.rpm` ne se mettent pas à jour : leurs formats passent par le gestionnaire de paquets du système. Sur Linux, c'est l'AppImage qui se remplace en place.

### Publier une version

```bash
node scripts/check-version.mjs 0.2.0   # écrit la version dans les trois fichiers
git commit -am "v0.2.0" && git tag v0.2.0 && git push --follow-tags
```

Le workflow `release` construit les installeurs sur Linux et Windows, les signe, publie `latest.json` à côté d'eux, et ne sort la release du brouillon qu'une fois les deux plateformes abouties.

## Configuration

Une seule chose à faire, une seule fois : coller un token Modrinth.

L'écran d'accueil ouvre directement [modrinth.com/settings/pats](https://modrinth.com/settings/pats) pour toi. Crée un token en cochant ces six autorisations, toutes en lecture seule :

`Read user data` · `Read notifications` · `Read payouts` · `Access analytics` · `Read projects` · `Read versions`

Colle-le, c'est terminé. Le token est validé auprès de Modrinth avant d'être confié au **trousseau du système** — le gestionnaire d'identifiants sous Windows, le service de secrets sous Linux — qui le chiffre avec les identifiants de ta session. Il ne quitte jamais le processus Rust et n'est jamais transmis à l'interface.

Aucun jeton n'est écrit dans le dossier de données : `session.json` ne garde que le pseudo et la date. Une installation antérieure qui les y avait laissés les voit passer au trousseau au premier démarrage, et le fichier est réécrit sans eux. Si le trousseau ne répond pas, rien n'est écrit en clair en repli — l'application le dit et redemande le jeton, plutôt que de reposer le secret sur le disque.

Se déconnecter efface les deux jetons, celui de Modrinth comme celui d'envoi CurseForge.

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
npm run version:check  # les trois fichiers de version doivent concorder
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

La webview tourne sous une politique de sécurité de contenu déclarée dans `tauri.conf.json` : aucun script distant, et les seules images extérieures admises sont les logos de mods servis par `cdn.modrinth.com` et `media.forgecdn.net`. Les styles en ligne restent autorisés — Svelte en pose sur les éléments qu'il anime.

Le design complet est dans [docs/superpowers/specs/2026-08-11-chartographer-design.md](docs/superpowers/specs/2026-08-11-chartographer-design.md).

## Licence

MIT
