# Chartographer, Design

Date : 2026-08-11
Statut : validé

## Problème

Les statistiques des mods Minecraft de l'auteur sont éclatées sur deux plateformes qui ne communiquent pas. Modrinth expose des séries temporelles riches (téléchargements, vues, revenus, pays) derrière une API authentifiée. CurseForge expose des totaux publics sans historique. Aucune vue ne permet de répondre à "où va réellement mon audience, et sur quelle plateforme".

Chartographer est une application de bureau qui agrège les deux sources, les croise projet par projet, et les rend sur une page de vision unique.

## Périmètre

Cockpit auteur, découverte entièrement automatique. Aucun identifiant de projet codé en dur : la liste des projets est dérivée des identités (token Modrinth, pseudo auteur CurseForge). Un mod publié après l'installation apparaît au cycle de synchronisation suivant.

Hors périmètre : publication de fichiers, édition de projets, exploration de mods tiers, comparaison avec des concurrents.

## Sources de données, état vérifié

### Modrinth

L'authentification se fait par **token personnel collé une fois**. En-tête `Authorization: <token>`, **sans** préfixe `Bearer`. Un `User-Agent` explicite est requis sur tous les appels.

OAuth2 a été implémenté puis retiré. Le flux fonctionnait, `GET /_internal/oauth/authorize` et `POST /_internal/oauth/token` existent, mais il impose une condition rédhibitoire : l'application doit être déclarée au préalable sur `modrinth.com/settings/applications`, et cette déclaration n'est **pas** automatisable. `POST /_internal/oauth/app` accepte bien un corps JSON (`name`, `max_scopes`, `redirect_uris`) mais répond `401 Invalid Authentication Credentials` avec un token personnel valide, alors que ce même token passe sur `/v2/user` : les routes de gestion d'applications exigent une session de navigateur. Demander à chaque utilisateur de créer sa propre application OAuth est une friction bien pire que coller un token, d'où l'abandon.

L'écran d'accueil ouvre `https://modrinth.com/settings/pats` pour l'utilisateur et liste les six portées à cocher, toutes en lecture seule : `Read user data`, `Read notifications`, `Read payouts`, `Access analytics`, `Read projects`, `Read versions`. Le token est validé par un appel à `/v2/user` avant d'être écrit sur le disque : une saisie erronée est rejetée immédiatement avec le message de l'API.

| Endpoint | Usage |
|---|---|
| `GET /v2/user` | identité, rôle, solde de paiement |
| `GET /v2/user/{id}/projects` | découverte automatique des projets |
| `GET /v2/project/{id}/version` | versions, loaders, versions de jeu, téléchargements |
| `GET /v2/user/{id}/notifications` | fil d'évènements |
| `GET /v3/analytics/downloads` | série temporelle par projet |
| `GET /v3/analytics/views` | série temporelle par projet |
| `GET /v3/analytics/revenue` | série temporelle par projet, montants en chaîne décimale |
| `GET /v3/analytics/countries/downloads` | répartition géographique |

Les endpoints d'analyse prennent `project_ids` (tableau JSON encodé en URL), `start_date`, `end_date`, `resolution_minutes`. Ils renvoient une map `project_id → { timestamp_unix_secondes → valeur }`. Les timestamps sont des chaînes dans les clés JSON. Les revenus arrivent en chaînes décimales de haute précision et doivent être parsés en décimal exact, jamais en flottant.

Note : `/v2/analytics/*` répond 404. Ces endpoints n'existent qu'en v3.

### CurseForge

**Aucune authentification.** CurseForge n'expose aucun flux OAuth, et il se trouve qu'il n'en faut pas : le token d'upload auteur initialement envisagé n'ouvre que `minecraft.curseforge.com/api/game/versions`, ce qui n'apporte aucune statistique. Il est écarté. La clé Core API sur `api.curseforge.com` répond 403 sans validation manuelle côté CurseForge et est écartée aussi.

Les statistiques viennent donc entièrement de CFWidget, public et sans authentification :

| Endpoint | Usage |
|---|---|
| `GET api.cfwidget.com/author/search/{username}` | découverte automatique : `{ id, username, projects: [{ id, name }] }` |
| `GET api.cfwidget.com/{projectId}` | titre, type, URLs, `downloads.total`, `downloads.monthly`, vignette, date de création |

CFWidget renvoie `202` lorsqu'une ressource n'est pas encore en cache et qu'un rafraîchissement est mis en file d'attente. Le client doit traiter ce cas comme "réessayer plus tard", pas comme une erreur.

Le seul paramètre CurseForge est le **pseudo auteur**, et il est déduit automatiquement sans jamais rien demander. L'application interroge `api.cfwidget.com/minecraft/mc-mods/{slug}` avec les slugs Modrinth déjà découverts et lit le tableau `members` de la réponse, dont l'entrée portant le titre `Owner` donne le pseudo réel, quel qu'il soit, même sans rapport avec le pseudo Modrinth. À défaut, elle essaie le pseudo Modrinth puis sa variante suffixée `_official`. Un champ de réglage permet de le corriger si tout échoue.

CFWidget ne fournit aucun historique. L'historique CurseForge est donc **construit localement** : chaque synchronisation écrit un snapshot horodaté du total, et les deltas entre snapshots produisent la courbe. Les premiers jours après installation, la courbe CurseForge est vide, c'est attendu et l'interface le signale explicitement plutôt que d'afficher un graphique trompeur.

## Architecture

Application Tauri v2. Tout le réseau et toute la persistance vivent côté Rust : la webview ne voit jamais le token et il n'y a aucun problème de CORS.

```
src-tauri/src/
  main.rs            point d'entrée, montage des commandes
  config.rs          chemins applicatifs, session, réglages
  error.rs           type d'erreur unifié, conversion vers le front
  providers/
    modrinth.rs      client HTTP Modrinth v2 + v3
    curseforge.rs    client CFWidget
    mod.rs           politique commune de retry et de rate-limit
  matching.rs        appariement Modrinth <-> CurseForge
  store/
    mod.rs           ouverture de la base
    schema.rs        migrations versionnées
    projects.rs      lecture/écriture projets et liens
    metrics.rs       lecture/écriture séries et snapshots
    queries.rs       agrégations alimentant la page de vision
  sync.rs            orchestration : découverte, rafraîchissement, snapshot
  commands.rs        surface exposée au front
```

Chaque module a une responsabilité unique et reste sous ~150 lignes. Les clients de provider ne connaissent pas la base ; le store ne connaît pas le réseau ; `sync.rs` est le seul point qui compose les deux.

Frontend Svelte 5 (runes) + TypeScript + Vite. Rendu des graphiques par Apache ECharts, retenu pour la carte géographique intégrée, le `dataZoom` par brush, et les heatmaps, les trois sont nécessaires ici et éviteraient sinon trois bibliothèques.

## Modèle de données

SQLite local, dans le dossier de données applicatif.

```sql
projects(
  id INTEGER PRIMARY KEY,
  platform TEXT NOT NULL,           -- 'modrinth' | 'curseforge'
  ext_id TEXT NOT NULL,             -- identifiant chez la plateforme
  slug TEXT,
  title TEXT NOT NULL,
  project_type TEXT,
  url TEXT,
  icon_url TEXT,
  created_at TEXT,
  total_downloads INTEGER,
  followers INTEGER,
  archived_at TEXT,                 -- non nul si le projet a disparu de la source
  UNIQUE(platform, ext_id)
)

links(
  modrinth_project_id INTEGER NOT NULL REFERENCES projects(id),
  cf_project_id INTEGER NOT NULL REFERENCES projects(id),
  confidence REAL NOT NULL,
  manual INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY(modrinth_project_id, cf_project_id)
)

metrics_daily(
  project_id INTEGER NOT NULL REFERENCES projects(id),
  day TEXT NOT NULL,                -- 'YYYY-MM-DD' UTC
  downloads INTEGER,
  views INTEGER,
  revenue TEXT,                     -- décimal exact en chaîne
  PRIMARY KEY(project_id, day)
)

countries_daily(
  project_id INTEGER NOT NULL REFERENCES projects(id),
  day TEXT NOT NULL,
  country TEXT NOT NULL,            -- ISO-2, 'XX' = inconnu, '' = non renseigné
  downloads INTEGER NOT NULL,
  PRIMARY KEY(project_id, day, country)
)

cf_snapshots(
  project_id INTEGER NOT NULL REFERENCES projects(id),
  taken_at TEXT NOT NULL,           -- ISO-8601 UTC
  total_downloads INTEGER NOT NULL,
  monthly_downloads INTEGER,
  PRIMARY KEY(project_id, taken_at)
)

versions(
  project_id INTEGER NOT NULL REFERENCES projects(id),
  version_id TEXT NOT NULL,
  version_number TEXT,
  game_versions TEXT,               -- JSON
  loaders TEXT,                     -- JSON
  downloads INTEGER,
  date_published TEXT,
  PRIMARY KEY(project_id, version_id)
)

events(
  id INTEGER PRIMARY KEY,
  source TEXT NOT NULL,
  occurred_at TEXT NOT NULL,
  kind TEXT NOT NULL,
  project_id INTEGER REFERENCES projects(id),
  payload TEXT
)

sync_runs(
  id INTEGER PRIMARY KEY,
  started_at TEXT NOT NULL,
  finished_at TEXT,
  provider TEXT NOT NULL,
  status TEXT NOT NULL,             -- 'ok' | 'partial' | 'failed'
  detail TEXT
)
```

Les projets qui disparaissent d'une source ne sont pas supprimés : `archived_at` est renseigné, l'historique reste consultable.

## Appariement inter-plateformes

Un mod publié sur les deux plateformes n'y porte pas forcément le même identifiant, ni le même slug. Cas réel constaté : Modrinth `mobsblocker` / "Mobs Blocker" face à CurseForge `mobblocker` / "Mobs Blocker". Modrinth `colony` / "Colony" face à CurseForge "Colony Project".

Algorithme, dans l'ordre, premier succès retenu :

1. Slug identique après normalisation (minuscules, suppression des séparateurs non alphanumériques).
2. Titre identique après normalisation.
3. Similarité de Jaro-Winkler sur les titres normalisés au-dessus de 0,88, avec un unique candidat au-dessus du seuil, l'ambiguïté ne produit jamais de lien automatique.
4. Sinon : non apparié. Le projet apparaît en mono-plateforme et l'interface propose un appariement manuel.

Un lien `manual = 1` n'est jamais écrasé par l'automatique. La confiance est stockée pour permettre à l'interface de signaler les liens fragiles.

## Synchronisation

Trois opérations distinctes, déclenchables séparément.

**Découverte**, interroge `/v2/user` et `/v2/user/{id}/projects` côté Modrinth, `author/search/{username}` côté CurseForge, insère ou met à jour `projects`, puis relance l'appariement. Le pseudo CurseForge est un réglage ; à la première installation l'application tente le pseudo Modrinth puis la variante suffixée `_official`, et demande confirmation.

**Rafraîchissement**, récupère les analyses Modrinth sur la fenêtre manquante (depuis le dernier jour connu, sinon depuis la création du premier projet), les versions, les notifications. Écrit dans `metrics_daily`, `countries_daily`, `versions`, `events`.

**Snapshot**, lit chaque projet CurseForge via CFWidget et écrit une ligne dans `cf_snapshots`. Au plus une fois par jour, au lancement de l'application, et sur demande.

Politique réseau : Modrinth limite à 300 requêtes par minute ; le client respecte les en-têtes `X-Ratelimit-Remaining` et `X-Ratelimit-Reset` et met en pause plutôt que d'encaisser un 429. Requêtes d'analyse groupées par lot de projets pour limiter le nombre d'appels. Retry exponentiel borné à trois tentatives sur erreurs réseau et 5xx ; pas de retry sur 4xx hors 429. CFWidget : `202` déclenche un unique nouvel essai différé, puis abandon silencieux du projet pour ce cycle.

Chaque provider échoue indépendamment. Une panne CurseForge n'empêche pas l'affichage des données Modrinth. Le résultat de chaque cycle est écrit dans `sync_runs` et l'interface affiche un badge de fraîcheur par source.

## Interface, la page de vision

Un écran unique, thème sombre par défaut, densité assumée. De haut en bas :

1. **Bandeau d'indicateurs**, téléchargements toutes plateformes, variation sur 30 jours, revenus cumulés et solde en attente, followers, nombre de projets actifs. Chaque indicateur porte sa micro-tendance.
2. **Aire empilée temporelle**, téléchargements par jour et par mod, bascule Modrinth / CurseForge / cumul, brush de zoom sur l'axe temporel qui pilote l'ensemble des autres graphiques de la page.
3. **Carte choroplèthe mondiale**, téléchargements par pays. `XX` et la chaîne vide sont agrégés dans une catégorie "inconnu" affichée hors carte, jamais fondus dans un pays réel.
4. **Barres comparatives par mod**, part Modrinth contre part CurseForge, triées par écart. C'est la vue qui répond à "où est vraiment mon public".
5. **Heatmap versions de jeu × loaders**, concentration des téléchargements.
6. **Courbe de revenus**, journalière et cumulée.
7. **Table de tous les projets**, triable, avec sparkline par ligne, badge de plateforme, indicateur de lien fragile. Clic sur une ligne : vue détaillée du mod.
8. **Fil d'évènements**, changements de statut, publications, notifications Modrinth.

Vue détaillée par mod : mêmes graphiques restreints au projet, plus la table de ses versions et l'écart de téléchargements entre plateformes.

Écran de réglages : état de la connexion Modrinth avec déconnexion et raccourci vers la page des tokens, pseudo CurseForge détecté et corrigeable, fenêtre d'historique, appariements manuels, purge et export de la base.

Premier lancement, écran unique : trois étapes numérotées, un bouton qui ouvre la page des tokens Modrinth, la liste des six portées à cocher, et un champ de collage.

Le thème suit `prefers-color-scheme` par défaut. Un bouton cycle entre automatique, clair et sombre ; le choix est mémorisé et l'emporte sur la préférence système dans les deux sens. Les palettes des graphiques dérivent du même état, donc axes, infobulles, carte et heatmap basculent avec le reste.

## Configuration et secrets

Une seule saisie, une seule fois : le token Modrinth. Le pseudo CurseForge se déduit tout seul.

Deux fichiers dans le dossier de données applicatif, aucun dans le dépôt :

| Fichier | Contenu |
|---|---|
| `session.json` | token Modrinth, date de saisie, pseudo et identifiant de l'utilisateur |
| `settings.json` | pseudo CurseForge s'il a fallu le corriger, fenêtre d'historique |

Le token ne franchit jamais la frontière vers la webview : les commandes Tauri renvoient un état de connexion et un pseudo, jamais la valeur. La déconnexion supprime `session.json` et purge l'état en mémoire.

Aucun token n'est journalisé. Les journaux sont actifs en développement uniquement, gatés par `#[cfg(debug_assertions)]` côté Rust et `import.meta.env.DEV` côté front.

## Gestion des erreurs

Un type d'erreur unifié côté Rust distingue quatre familles : configuration absente ou invalide, authentification refusée, indisponibilité réseau ou distante, et incohérence de données. Chacune se traduit par un message actionnable côté interface, "token Modrinth refusé, vérifie les réglages" plutôt qu'une trace brute.

Les échecs partiels sont la norme, pas l'exception : l'interface affiche toujours ce qui a pu être chargé, avec l'âge de chaque source.

## Tests

Tests unitaires Rust sur l'appariement (les cas réels `mobsblocker`/`mobblocker` et `colony`/`Colony Project` sont des cas de test), sur le parsing des réponses d'analyse à partir de fixtures JSON enregistrées, sur l'extraction du propriétaire CFWidget, et sur l'arithmétique décimale des revenus. Tests d'intégration du store sur base SQLite en mémoire, incluant les migrations et l'idempotence des écritures. Tests front Vitest sur les transformations de séries alimentant les graphiques.

Aucun appel réseau réel en intégration continue : toutes les réponses distantes sont des fixtures.

## Distribution

GitHub Actions, déclenchement sur tag `v*`. Matrice : `ubuntu-latest` produit `.deb` et `.rpm`, `windows-latest` produit l'installeur NSIS `.exe`. Les artefacts sont attachés à une Release GitHub. Un workflow de contrôle sur push exécute `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `svelte-check` et `vitest`.

## Versions retenues

Rust : tauri 2.11, tauri-build 2.6, tauri-plugin-opener 2.5, rusqlite 0.40 (feature `bundled`), reqwest 0.13, tokio 1.53, serde 1.0, chrono 0.4, rust_decimal 1.42, strsim 0.11, thiserror 2.0, anyhow 1.0, tracing 0.1.

Node : @tauri-apps/cli 2.11, @tauri-apps/api 2.11, svelte 5.56, vite 8.2, echarts 6.1, typescript 7.0, vitest 4.1.

## Décisions écartées

Electron : trois fois le poids de binaire pour un gain nul ici, et les identifiants exposés au processus de rendu.

Clé CurseForge Core API : demanderait une validation manuelle côté CurseForge que l'auteur n'a pas. CFWidget couvre le besoin de lecture.

Token d'upload CurseForge : n'ouvre que le catalogue des versions de jeu, aucune statistique. Supprimé du périmètre, ce qui retire toute authentification côté CurseForge.

**OAuth2 Modrinth : implémenté puis retiré.** Le flux marchait, mais il exige que chaque personne installant l'application déclare au préalable une application OAuth sur son compte Modrinth, la création par API est fermée aux tokens personnels (`401` vérifié). Faire créer une application OAuth à un utilisateur est une friction bien supérieure à un copier-coller de token. Le code correspondant a été supprimé plutôt que laissé en repli mort.

Webview interne pour une page de connexion : écartée dans tous les cas. Faire saisir un mot de passe Modrinth dans une webview applicative est un anti-patron.

Stockage du token dans le trousseau système : écarté au profit d'un `session.json` dans le dossier de données applicatif, plus simple à purger et à déboguer.
