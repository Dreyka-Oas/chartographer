# Chartographer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Une application de bureau qui agrège les statistiques de mods Minecraft de Modrinth et CurseForge, les croise projet par projet, et les rend sur une page de vision unique, distribuée en `.deb`, `.rpm` et `.exe`.

**Architecture :** Tauri v2. Tout le réseau et toute la persistance vivent côté Rust, la webview ne voit jamais les tokens. SQLite local conserve les séries Modrinth et reconstruit l'historique CurseForge par snapshots quotidiens. Le front Svelte 5 lit un seul DTO agrégé et le rend avec Apache ECharts.

**Tech Stack :** Rust 1.95, tauri 2.11, rusqlite 0.40 (bundled), reqwest 0.13, tokio 1.53, rust_decimal 1.42, strsim 0.11 — Node 24, Svelte 5.56, Vite 8.2, TypeScript 5.9, ECharts 6.1, Vitest 4.1.

**Spec :** `docs/superpowers/specs/2026-08-11-chartographer-design.md`

---

## Structure des fichiers

```
src-tauri/src/
  main.rs              point d'entrée binaire
  lib.rs               montage Tauri, état applicatif
  error.rs             AppError + sérialisation vers le front
  config.rs            .env applicatif, lecture/écriture des identifiants
  models.rs            Platform + DTO partagés Rust <-> front
  matching.rs          appariement Modrinth <-> CurseForge
  providers/
    mod.rs             politique de retry et de rate-limit
    modrinth.rs        client v2 + v3, parsing pur testable
    curseforge.rs      client CFWidget + vérification du token d'upload
  store/
    mod.rs             Store, ouverture, transaction
    schema.rs          migrations versionnées
    projects.rs        projets et liens
    metrics.rs         séries, pays, snapshots, versions, évènements
    queries.rs         agrégations alimentant l'Overview
  sync.rs              découverte, rafraîchissement, snapshot
  commands.rs          surface exposée au front

src/
  main.ts              montage Svelte
  App.svelte           routeur de vues
  app.css              tokens de thème
  lib/
    api.ts             wrappers typés d'invoke
    types.ts           types miroir des DTO Rust
    state.svelte.ts    état global en runes
    format.ts          formatage nombres, dates, devises
    charts/
      Chart.svelte     wrapper ECharts
      timeline.ts      construction d'option, pur
      worldmap.ts      construction d'option, pur
      split.ts         construction d'option, pur
      heatmap.ts       construction d'option, pur
      revenue.ts       construction d'option, pur
      sparkline.ts     construction d'option, pur
    components/
      KpiBand.svelte
      Timeline.svelte
      WorldMap.svelte
      PlatformSplit.svelte
      LoaderHeatmap.svelte
      RevenueChart.svelte
      ProjectsTable.svelte
      EventsFeed.svelte
      FreshnessBadge.svelte
    views/
      Vision.svelte
      ProjectDetail.svelte
      Settings.svelte
```

---

### Task 1 : Squelette Tauri v2 + Svelte 5

**Files:**
- Create: `package.json`, `vite.config.ts`, `tsconfig.json`, `tsconfig.node.json`, `svelte.config.js`, `index.html`, `src/main.ts`, `src/App.svelte`, `src/app.css`, `src/vite-env.d.ts`
- Create: `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1 : `package.json`**

```json
{
  "name": "chartographer",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "check": "svelte-check --tsconfig ./tsconfig.json",
    "test": "vitest run",
    "tauri": "tauri"
  },
  "dependencies": {
    "@tauri-apps/api": "2.11.1",
    "echarts": "6.1.0",
    "topojson-client": "3.1.0",
    "world-atlas": "2.0.2"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "7.3.0",
    "@tauri-apps/cli": "2.11.4",
    "@tsconfig/svelte": "5.0.8",
    "@types/topojson-client": "3.1.5",
    "svelte": "5.56.8",
    "svelte-check": "4.7.5",
    "typescript": "5.9.3",
    "vite": "8.2.1",
    "vitest": "4.1.10"
  }
}
```

- [ ] **Step 2 : `vite.config.ts`**

```ts
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, watch: { ignored: ["**/src-tauri/**"] } },
  build: { target: "es2022", sourcemap: false },
});
```

- [ ] **Step 3 : `tsconfig.json`, `tsconfig.node.json`, `svelte.config.js`**

`tsconfig.json` :

```json
{
  "extends": "@tsconfig/svelte/tsconfig.json",
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "resolveJsonModule": true,
    "strict": true,
    "noUnusedLocals": true,
    "types": ["vite/client"]
  },
  "include": ["src/**/*.ts", "src/**/*.svelte"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

`tsconfig.node.json` :

```json
{
  "compilerOptions": {
    "composite": true,
    "skipLibCheck": true,
    "module": "ESNext",
    "moduleResolution": "bundler"
  },
  "include": ["vite.config.ts"]
}
```

`svelte.config.js` :

```js
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
export default { preprocess: vitePreprocess() };
```

- [ ] **Step 4 : `index.html`, `src/main.ts`, `src/App.svelte`, `src/vite-env.d.ts`**

`index.html` :

```html
<!doctype html>
<html lang="fr">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Chartographer</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

`src/main.ts` :

```ts
import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";

export default mount(App, { target: document.getElementById("app")! });
```

`src/App.svelte` :

```svelte
<script lang="ts">
  let view = $state<"vision" | "settings">("vision");
</script>

<main>
  <h1>Chartographer</h1>
  <p>Vue courante : {view}</p>
</main>
```

`src/vite-env.d.ts` :

```ts
/// <reference types="svelte" />
/// <reference types="vite/client" />
```

- [ ] **Step 5 : `src/app.css` — tokens de thème sombre**

```css
:root {
  color-scheme: dark;
  --bg: #0d1013;
  --surface: #14181d;
  --surface-2: #1b2027;
  --border: #262d36;
  --text: #e6ebf0;
  --text-dim: #8b97a5;
  --accent: #5ac8a8;
  --modrinth: #00af5c;
  --curseforge: #f16436;
  --warn: #e0a458;
  --error: #e05c5c;
  --radius: 10px;
  font-family: ui-sans-serif, system-ui, "Segoe UI", sans-serif;
}
* { box-sizing: border-box; }
body { margin: 0; background: var(--bg); color: var(--text); }
```

- [ ] **Step 6 : `src-tauri/Cargo.toml`**

```toml
[package]
name = "chartographer"
version = "0.1.0"
edition = "2021"
rust-version = "1.85"

[lib]
name = "chartographer_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2.6", features = [] }

[dependencies]
tauri = { version = "2.11", features = [] }
tauri-plugin-opener = "2.5"
tokio = { version = "1.53", features = ["rt-multi-thread", "macros", "time", "sync", "net", "io-util"] }
reqwest = { version = "0.13", features = ["json"] }
rusqlite = { version = "0.40", features = ["bundled"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
rust_decimal = { version = "1.42", features = ["serde-with-str"] }
strsim = "0.11"
thiserror = "2.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[profile.release]
opt-level = "s"
lto = true
strip = true
```

- [ ] **Step 7 : `src-tauri/build.rs`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`**

`build.rs` :

```rust
fn main() {
    tauri_build::build()
}
```

`src/main.rs` :

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    chartographer_lib::run()
}
```

`src/lib.rs` :

```rust
pub fn run() {
    #[cfg(debug_assertions)]
    tracing_subscriber::fmt().with_env_filter("chartographer_lib=debug").init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("erreur au démarrage de Tauri");
}
```

- [ ] **Step 8 : `src-tauri/tauri.conf.json`**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Chartographer",
  "version": "0.1.0",
  "identifier": "fr.dreykaoas.chartographer",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "Chartographer",
        "width": 1560,
        "height": 980,
        "minWidth": 1100,
        "minHeight": 700,
        "resizable": true
      }
    ],
    "security": { "csp": null }
  },
  "bundle": {
    "active": true,
    "targets": ["deb", "rpm", "nsis"],
    "category": "Utility",
    "shortDescription": "Statistiques de mods Modrinth et CurseForge",
    "longDescription": "Cockpit de bureau qui agrege les statistiques de mods Minecraft publiees sur Modrinth et CurseForge sur une page de vision unique.",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "linux": { "deb": { "depends": [] } }
  }
}
```

- [ ] **Step 9 : `src-tauri/capabilities/default.json`**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capacites de la fenetre principale",
  "windows": ["main"],
  "permissions": ["core:default", "opener:default", "opener:allow-open-url"]
}
```

- [ ] **Step 10 : générer l'icône source puis les déclinaisons**

Génère un PNG 1024×1024 sans dépendance externe, puis laisse la CLI Tauri produire toutes les tailles.

```powershell
Add-Type -AssemblyName System.Drawing
$bmp = New-Object System.Drawing.Bitmap 1024,1024
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = 'AntiAlias'
$g.Clear([System.Drawing.Color]::FromArgb(255,13,16,19))
$pen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255,90,200,168)), 46
$pts = @(
  (New-Object System.Drawing.Point 150,760), (New-Object System.Drawing.Point 360,520),
  (New-Object System.Drawing.Point 560,620), (New-Object System.Drawing.Point 780,240)
)
$g.DrawLines($pen, $pts)
$brush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255,241,100,54))
foreach ($p in $pts) { $g.FillEllipse($brush, $p.X-34, $p.Y-34, 68, 68) }
$g.Dispose()
New-Item -ItemType Directory -Force -Path 'src-tauri/icons' | Out-Null
$bmp.Save((Resolve-Path 'src-tauri/icons').Path + '\source.png', [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
```

Puis :

```powershell
npm install
npx tauri icon src-tauri/icons/source.png
```

Attendu : `src-tauri/icons/` contient `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.ico`, `icon.icns`.

- [ ] **Step 11 : vérifier que ça compile et démarre**

```powershell
npm run check
cargo check --manifest-path src-tauri/Cargo.toml
```

Attendu : `svelte-check` sort sans erreur, `cargo check` sort `Finished`.

- [ ] **Step 12 : commit**

```powershell
git add -A
git commit -m "feat: squelette Tauri v2 + Svelte 5"
```

---

### Task 2 : Type d'erreur et configuration

**Files:**
- Create: `src-tauri/src/error.rs`, `src-tauri/src/config.rs`
- Modify: `src-tauri/src/lib.rs`

Il n'y a plus de fichier `.env` ni de token saisi à la main. Trois fichiers JSON dans le dossier de données applicatif : `session.json` (token OAuth obtenu), `oauth.json` (identifiants d'application, uniquement si le binaire a été compilé sans), `settings.json` (pseudo CurseForge corrigé, fenêtre d'historique).

- [ ] **Step 1 : écrire les tests qui échouent**

Ajoute en bas de `src-tauri/src/config.rs` (le fichier n'existe pas encore, crée-le avec le seul bloc de test pour l'instant) :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("chartographer-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn session_roundtrips_on_disk() {
        let dir = tmp();
        assert!(load_session(&dir).is_none());
        let session = Session {
            token: "mrp_abc".into(),
            user_id: "VgD9obZq".into(),
            username: "DreykaOas".into(),
            obtained_at: "2026-08-11T10:00:00Z".into(),
        };
        save_session(&dir, &session).unwrap();
        assert_eq!(load_session(&dir).unwrap(), session);
        clear_session(&dir).unwrap();
        assert!(load_session(&dir).is_none());
    }

    #[test]
    fn settings_default_when_absent_then_persist() {
        let dir = tmp().join("settings");
        let defaults = load_settings(&dir);
        assert_eq!(defaults.range_days, 90);
        assert!(defaults.curseforge_username.is_none());

        let updated = Settings { curseforge_username: Some("DreykaOas_official".into()), range_days: 180 };
        save_settings(&dir, &updated).unwrap();
        assert_eq!(load_settings(&dir), updated);
    }

    #[test]
    fn oauth_app_prefers_compiled_values_then_disk() {
        let dir = tmp().join("oauth");
        // Sans valeurs compilees ni fichier, aucune application n'est configuree.
        assert!(load_oauth_app(&dir, None, None).is_none());

        save_oauth_app(&dir, &OauthApp { client_id: "disk".into(), client_secret: "s1".into() }).unwrap();
        assert_eq!(load_oauth_app(&dir, None, None).unwrap().client_id, "disk");

        // Les valeurs compilees priment toujours sur le fichier.
        let compiled = load_oauth_app(&dir, Some("compiled"), Some("s2")).unwrap();
        assert_eq!(compiled.client_id, "compiled");
        assert_eq!(compiled.client_secret, "s2");
    }

    #[test]
    fn oauth_app_ignores_half_filled_compiled_values() {
        let dir = tmp().join("oauth-partial");
        assert!(load_oauth_app(&dir, Some("only-id"), None).is_none());
        assert!(load_oauth_app(&dir, None, Some("only-secret")).is_none());
    }
}
```

- [ ] **Step 2 : lancer le test pour vérifier qu'il échoue**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml config
```

Attendu : échec de compilation, `cannot find type Session`.

- [ ] **Step 3 : écrire `src-tauri/src/error.rs`**

```rust
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("configuration manquante ou invalide : {0}")]
    Config(String),
    #[error("{provider} a refusé l'authentification : {detail}")]
    Auth { provider: String, detail: String },
    #[error("{provider} indisponible : {detail}")]
    Remote { provider: String, detail: String },
    #[error("données incohérentes : {0}")]
    Data(String),
}

impl AppError {
    pub fn kind(&self) -> &'static str {
        match self {
            AppError::Config(_) => "config",
            AppError::Auth { .. } => "auth",
            AppError::Remote { .. } => "remote",
            AppError::Data(_) => "data",
        }
    }

    pub fn remote(provider: &str, detail: impl Into<String>) -> Self {
        AppError::Remote { provider: provider.into(), detail: detail.into() }
    }

    pub fn auth(provider: &str, detail: impl Into<String>) -> Self {
        AppError::Auth { provider: provider.into(), detail: detail.into() }
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("AppError", 2)?;
        st.serialize_field("kind", self.kind())?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Data(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Data(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
```

- [ ] **Step 4 : écrire `src-tauri/src/config.rs` au-dessus du bloc de test**

```rust
use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Token Modrinth obtenu par OAuth. Ne franchit jamais la frontière vers la webview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub token: String,
    pub user_id: String,
    pub username: String,
    pub obtained_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub curseforge_username: Option<String>,
    #[serde(default = "default_range_days")]
    pub range_days: i64,
}

fn default_range_days() -> i64 {
    90
}

impl Default for Settings {
    fn default() -> Self {
        Settings { curseforge_username: None, range_days: default_range_days() }
    }
}

/// Identifiants de l'application OAuth enregistrée sur modrinth.com/settings/applications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OauthApp {
    pub client_id: String,
    pub client_secret: String,
}

pub fn db_path(app_data: &Path) -> PathBuf {
    app_data.join("chartographer.db")
}

fn session_path(app_data: &Path) -> PathBuf {
    app_data.join("session.json")
}

fn settings_path(app_data: &Path) -> PathBuf {
    app_data.join("settings.json")
}

fn oauth_path(app_data: &Path) -> PathBuf {
    app_data.join("oauth.json")
}

fn read_json<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Option<T> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn write_json<T: Serialize>(app_data: &Path, path: PathBuf, value: &T) -> Result<()> {
    std::fs::create_dir_all(app_data)
        .map_err(|e| AppError::Config(format!("dossier de configuration : {e}")))?;
    let raw = serde_json::to_string_pretty(value)?;
    std::fs::write(path, raw).map_err(|e| AppError::Config(format!("écriture : {e}")))
}

pub fn load_session(app_data: &Path) -> Option<Session> {
    read_json(session_path(app_data))
}

pub fn save_session(app_data: &Path, session: &Session) -> Result<()> {
    write_json(app_data, session_path(app_data), session)
}

pub fn clear_session(app_data: &Path) -> Result<()> {
    match std::fs::remove_file(session_path(app_data)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AppError::Config(format!("suppression de la session : {e}"))),
    }
}

pub fn require_token(app_data: &Path) -> Result<Session> {
    load_session(app_data)
        .ok_or_else(|| AppError::Config("aucune session Modrinth, connecte-toi d'abord".into()))
}

pub fn load_settings(app_data: &Path) -> Settings {
    read_json(settings_path(app_data)).unwrap_or_default()
}

pub fn save_settings(app_data: &Path, settings: &Settings) -> Result<()> {
    write_json(app_data, settings_path(app_data), settings)
}

/// Les valeurs injectées à la compilation priment sur le fichier.
/// Un couple incomplet est ignoré : il vaut mieux aucune application qu'une application cassée.
pub fn load_oauth_app(
    app_data: &Path,
    compiled_id: Option<&str>,
    compiled_secret: Option<&str>,
) -> Option<OauthApp> {
    match (compiled_id, compiled_secret) {
        (Some(id), Some(secret)) if !id.is_empty() && !secret.is_empty() => {
            return Some(OauthApp { client_id: id.into(), client_secret: secret.into() })
        }
        _ => {}
    }
    read_json(oauth_path(app_data))
}

pub fn save_oauth_app(app_data: &Path, app: &OauthApp) -> Result<()> {
    write_json(app_data, oauth_path(app_data), app)
}
```

- [ ] **Step 5 : déclarer les modules dans `src-tauri/src/lib.rs`**

Ajoute en tête du fichier :

```rust
pub mod config;
pub mod error;
```

- [ ] **Step 6 : lancer les tests**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml config
```

Attendu : `test result: ok. 4 passed`.

- [ ] **Step 7 : commit**

```powershell
git add -A
git commit -m "feat: type d'erreur unifie, session OAuth et reglages sur disque"
```

---

### Task 3 : Modèles partagés

**Files:**
- Create: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1 : écrire le test qui échoue**

Dans `src-tauri/src/models.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_roundtrips_through_str() {
        assert_eq!(Platform::from_str_lossy("modrinth"), Platform::Modrinth);
        assert_eq!(Platform::from_str_lossy("curseforge"), Platform::CurseForge);
        assert_eq!(Platform::Modrinth.as_str(), "modrinth");
        assert_eq!(Platform::CurseForge.as_str(), "curseforge");
    }
}
```

- [ ] **Step 2 : lancer le test pour vérifier qu'il échoue**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml models
```

Attendu : `cannot find type Platform`.

- [ ] **Step 3 : écrire le module au-dessus du bloc de test**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Modrinth,
    CurseForge,
}

impl Platform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Platform::Modrinth => "modrinth",
            Platform::CurseForge => "curseforge",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "curseforge" => Platform::CurseForge,
            _ => Platform::Modrinth,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Kpis {
    pub downloads_total: i64,
    pub downloads_modrinth: i64,
    pub downloads_curseforge: i64,
    pub downloads_30d: i64,
    pub downloads_prev_30d: i64,
    pub revenue_total: String,
    pub revenue_pending: String,
    pub followers: i64,
    pub projects_active: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelinePoint {
    pub day: String,
    pub modrinth: i64,
    pub curseforge: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectSummary {
    pub key: String,
    pub title: String,
    pub icon_url: Option<String>,
    pub modrinth_id: Option<i64>,
    pub curseforge_id: Option<i64>,
    pub modrinth_downloads: i64,
    pub curseforge_downloads: i64,
    pub followers: i64,
    pub link_confidence: Option<f64>,
    pub spark: Vec<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CountryTotal {
    pub country: String,
    pub downloads: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoaderCell {
    pub game_version: String,
    pub loader: String,
    pub downloads: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RevenuePoint {
    pub day: String,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventRow {
    pub occurred_at: String,
    pub kind: String,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Freshness {
    pub provider: String,
    pub status: String,
    pub finished_at: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Overview {
    pub kpis: Kpis,
    pub timeline: Vec<TimelinePoint>,
    pub per_project: Vec<ProjectSummary>,
    pub countries: Vec<CountryTotal>,
    pub loaders: Vec<LoaderCell>,
    pub revenue: Vec<RevenuePoint>,
    pub events: Vec<EventRow>,
    pub freshness: Vec<Freshness>,
    pub curseforge_history_days: i64,
}
```

- [ ] **Step 4 : déclarer le module**

Dans `src-tauri/src/lib.rs`, ajoute `pub mod models;`.

- [ ] **Step 5 : lancer les tests**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml models
```

Attendu : `test result: ok. 1 passed`.

- [ ] **Step 6 : commit**

```powershell
git add -A
git commit -m "feat: modeles partages entre Rust et le front"
```

---

### Task 4 : Appariement inter-plateformes

**Files:**
- Create: `src-tauri/src/matching.rs`
- Modify: `src-tauri/src/lib.rs`

Les cas de test sont réels : Modrinth `mobsblocker` / « Mobs Blocker » face à CurseForge `mobblocker` / « Mobs Blocker », et Modrinth `colony` / « Colony » face à CurseForge « Colony Project ».

- [ ] **Step 1 : écrire les tests qui échouent**

Dans `src-tauri/src/matching.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn c(id: i64, slug: &str, title: &str) -> Candidate {
        Candidate { id, slug: Some(slug.into()), title: title.into() }
    }

    #[test]
    fn normalize_strips_separators_and_case() {
        assert_eq!(normalize("Custom Clear Lag"), "customclearlag");
        assert_eq!(normalize("no-night-skip"), "nonightskip");
        assert_eq!(normalize("Vein_Vantage"), "veinvantage");
    }

    #[test]
    fn exact_slug_wins() {
        let m = vec![c(1, "vein-vantage", "Vein Vantage")];
        let cf = vec![c(10, "vein-vantage", "Something Else")];
        let out = match_projects(&m, &cf);
        assert_eq!(out.len(), 1);
        assert_eq!((out[0].modrinth_id, out[0].curseforge_id), (1, 10));
        assert_eq!(out[0].confidence, 1.0);
    }

    #[test]
    fn exact_title_matches_when_slugs_differ() {
        let m = vec![c(1, "mobsblocker", "Mobs Blocker")];
        let cf = vec![c(10, "mobblocker", "Mobs Blocker")];
        let out = match_projects(&m, &cf);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].confidence, 1.0);
    }

    #[test]
    fn fuzzy_title_matches_colony_project() {
        let m = vec![c(1, "colony", "Colony")];
        let cf = vec![c(10, "colony-project", "Colony Project")];
        let out = match_projects(&m, &cf);
        assert_eq!(out.len(), 1);
        assert!(out[0].confidence >= 0.88 && out[0].confidence < 1.0);
    }

    #[test]
    fn ambiguous_fuzzy_produces_no_match() {
        let m = vec![c(1, "colony", "Colony")];
        let cf = vec![c(10, "colonies", "Colonies"), c(11, "colonyx", "Colony X")];
        assert!(match_projects(&m, &cf).is_empty());
    }

    #[test]
    fn unrelated_projects_do_not_match() {
        let m = vec![c(1, "fake-fps", "Fake FPS")];
        let cf = vec![c(10, "zone-cleaner", "Zone Cleaner")];
        assert!(match_projects(&m, &cf).is_empty());
    }

    #[test]
    fn each_curseforge_project_is_claimed_once() {
        let m = vec![c(1, "health-tag", "Health Tag"), c(2, "healthtag", "Health Tag")];
        let cf = vec![c(10, "health-tag", "Health Tag")];
        assert_eq!(match_projects(&m, &cf).len(), 1);
    }
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml matching
```

Attendu : `cannot find function normalize`.

- [ ] **Step 3 : écrire l'implémentation au-dessus du bloc de test**

```rust
use std::collections::HashSet;

pub const FUZZY_THRESHOLD: f64 = 0.88;

#[derive(Debug, Clone)]
pub struct Candidate {
    pub id: i64,
    pub slug: Option<String>,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Match {
    pub modrinth_id: i64,
    pub curseforge_id: i64,
    pub confidence: f64,
}

pub fn normalize(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Apparie chaque projet Modrinth avec au plus un projet CurseForge.
/// Slug exact, puis titre exact, puis Jaro-Winkler sans ambiguïté.
pub fn match_projects(modrinth: &[Candidate], curseforge: &[Candidate]) -> Vec<Match> {
    let mut claimed: HashSet<i64> = HashSet::new();
    let mut out = Vec::new();

    for pass in 0..3 {
        for m in modrinth {
            if out.iter().any(|x: &Match| x.modrinth_id == m.id) {
                continue;
            }
            let found = match pass {
                0 => exact(m, curseforge, &claimed, |c| c.slug.as_deref().unwrap_or_default()),
                1 => exact(m, curseforge, &claimed, |c| c.title.as_str()),
                _ => fuzzy(m, curseforge, &claimed),
            };
            if let Some((cf_id, confidence)) = found {
                claimed.insert(cf_id);
                out.push(Match { modrinth_id: m.id, curseforge_id: cf_id, confidence });
            }
        }
    }
    out
}

fn exact<F>(m: &Candidate, pool: &[Candidate], claimed: &HashSet<i64>, field: F) -> Option<(i64, f64)>
where
    F: Fn(&Candidate) -> &str,
{
    let needle = normalize(field(m));
    if needle.is_empty() {
        return None;
    }
    pool.iter()
        .find(|c| !claimed.contains(&c.id) && normalize(field(c)) == needle)
        .map(|c| (c.id, 1.0))
}

fn fuzzy(m: &Candidate, pool: &[Candidate], claimed: &HashSet<i64>) -> Option<(i64, f64)> {
    let needle = normalize(&m.title);
    let mut hits: Vec<(i64, f64)> = pool
        .iter()
        .filter(|c| !claimed.contains(&c.id))
        .map(|c| (c.id, strsim::jaro_winkler(&needle, &normalize(&c.title))))
        .filter(|(_, score)| *score >= FUZZY_THRESHOLD)
        .collect();

    if hits.len() != 1 {
        return None;
    }
    let (id, score) = hits.pop().unwrap();
    Some((id, score.min(0.999)))
}
```

- [ ] **Step 4 : déclarer le module**

Dans `src-tauri/src/lib.rs`, ajoute `pub mod matching;`.

- [ ] **Step 5 : lancer les tests**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml matching
```

Attendu : `test result: ok. 7 passed`.

- [ ] **Step 6 : commit**

```powershell
git add -A
git commit -m "feat: appariement Modrinth vers CurseForge par slug, titre et similarite"
```

---

### Task 5 : Schéma SQLite et migrations

**Files:**
- Create: `src-tauri/src/store/mod.rs`, `src-tauri/src/store/schema.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1 : écrire les tests qui échouent**

Dans `src-tauri/src/store/schema.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn version(conn: &Connection) -> i64 {
        conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn migrate_creates_all_tables() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        let names: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        for expected in [
            "cf_snapshots", "countries_daily", "events", "links", "meta",
            "metrics_daily", "projects", "sync_runs", "versions",
        ] {
            assert!(names.contains(&expected.to_string()), "table manquante : {expected}");
        }
    }

    #[test]
    fn migrate_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        let first = version(&conn);
        migrate(&conn).unwrap();
        assert_eq!(version(&conn), first);
    }

    #[test]
    fn migrate_sets_user_version() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        assert_eq!(version(&conn), SCHEMA_VERSION);
    }
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml schema
```

Attendu : `cannot find function migrate`.

- [ ] **Step 3 : écrire `src-tauri/src/store/schema.rs` au-dessus du bloc de test**

```rust
use crate::error::Result;
use rusqlite::Connection;

pub const SCHEMA_VERSION: i64 = 1;

const V1: &str = r#"
CREATE TABLE projects (
  id INTEGER PRIMARY KEY,
  platform TEXT NOT NULL,
  ext_id TEXT NOT NULL,
  slug TEXT,
  title TEXT NOT NULL,
  project_type TEXT,
  url TEXT,
  icon_url TEXT,
  created_at TEXT,
  total_downloads INTEGER NOT NULL DEFAULT 0,
  followers INTEGER NOT NULL DEFAULT 0,
  archived_at TEXT,
  UNIQUE(platform, ext_id)
);

CREATE TABLE links (
  modrinth_project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  cf_project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  confidence REAL NOT NULL,
  manual INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY(modrinth_project_id, cf_project_id)
);

CREATE TABLE metrics_daily (
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  day TEXT NOT NULL,
  downloads INTEGER,
  views INTEGER,
  revenue TEXT,
  PRIMARY KEY(project_id, day)
);

CREATE TABLE countries_daily (
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  day TEXT NOT NULL,
  country TEXT NOT NULL,
  downloads INTEGER NOT NULL,
  PRIMARY KEY(project_id, day, country)
);

CREATE TABLE cf_snapshots (
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  taken_at TEXT NOT NULL,
  total_downloads INTEGER NOT NULL,
  monthly_downloads INTEGER,
  PRIMARY KEY(project_id, taken_at)
);

CREATE TABLE versions (
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  version_id TEXT NOT NULL,
  version_number TEXT,
  game_versions TEXT,
  loaders TEXT,
  downloads INTEGER NOT NULL DEFAULT 0,
  date_published TEXT,
  PRIMARY KEY(project_id, version_id)
);

CREATE TABLE events (
  id INTEGER PRIMARY KEY,
  source TEXT NOT NULL,
  occurred_at TEXT NOT NULL,
  kind TEXT NOT NULL,
  project_id INTEGER REFERENCES projects(id) ON DELETE SET NULL,
  title TEXT NOT NULL,
  detail TEXT NOT NULL DEFAULT '',
  UNIQUE(source, occurred_at, kind, title)
);

CREATE TABLE sync_runs (
  id INTEGER PRIMARY KEY,
  started_at TEXT NOT NULL,
  finished_at TEXT,
  provider TEXT NOT NULL,
  status TEXT NOT NULL,
  detail TEXT NOT NULL DEFAULT ''
);

CREATE TABLE meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE INDEX idx_metrics_day ON metrics_daily(day);
CREATE INDEX idx_countries_day ON countries_daily(day);
CREATE INDEX idx_snapshots_taken ON cf_snapshots(taken_at);
CREATE INDEX idx_events_occurred ON events(occurred_at DESC);
"#;

pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;

    if current < 1 {
        conn.execute_batch(V1)?;
    }

    if current < SCHEMA_VERSION {
        conn.execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION};"))?;
    }
    Ok(())
}
```

- [ ] **Step 4 : écrire `src-tauri/src/store/mod.rs`**

```rust
pub mod schema;

use crate::error::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode = WAL;")?;
        schema::migrate(&conn)?;
        Ok(Store { conn: Mutex::new(conn) })
    }

    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        schema::migrate(&conn)?;
        Ok(Store { conn: Mutex::new(conn) })
    }

    /// Exécute une opération sous verrou. Le verrou est empoisonné uniquement
    /// si une opération a paniqué, ce qui est un bug — on propage la panique.
    pub fn with<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let guard = self.conn.lock().expect("verrou de base empoisonné");
        f(&guard)
    }
}
```

- [ ] **Step 5 : déclarer le module**

Dans `src-tauri/src/lib.rs`, ajoute `pub mod store;`.

- [ ] **Step 6 : lancer les tests**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml schema
```

Attendu : `test result: ok. 3 passed`.

- [ ] **Step 7 : commit**

```powershell
git add -A
git commit -m "feat: schema SQLite et migrations versionnees"
```

---

### Task 6 : Accès aux projets et aux liens

**Files:**
- Create: `src-tauri/src/store/projects.rs`
- Modify: `src-tauri/src/store/mod.rs`

- [ ] **Step 1 : écrire les tests qui échouent**

Dans `src-tauri/src/store/projects.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::schema::migrate;
    use rusqlite::Connection;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        conn
    }

    fn sample(platform: Platform, ext: &str, title: &str) -> ProjectUpsert {
        ProjectUpsert {
            platform,
            ext_id: ext.into(),
            slug: Some(title.to_lowercase().replace(' ', "-")),
            title: title.into(),
            project_type: Some("mod".into()),
            url: None,
            icon_url: None,
            created_at: None,
            total_downloads: 100,
            followers: 2,
        }
    }

    #[test]
    fn upsert_inserts_then_updates_same_row() {
        let conn = db();
        let id = upsert(&conn, &sample(Platform::Modrinth, "abc", "Vein Vantage")).unwrap();
        let mut second = sample(Platform::Modrinth, "abc", "Vein Vantage");
        second.total_downloads = 999;
        let id2 = upsert(&conn, &second).unwrap();
        assert_eq!(id, id2);
        assert_eq!(list(&conn).unwrap().len(), 1);
        assert_eq!(list(&conn).unwrap()[0].total_downloads, 999);
    }

    #[test]
    fn same_ext_id_on_two_platforms_are_distinct_rows() {
        let conn = db();
        upsert(&conn, &sample(Platform::Modrinth, "abc", "A")).unwrap();
        upsert(&conn, &sample(Platform::CurseForge, "abc", "A")).unwrap();
        assert_eq!(list(&conn).unwrap().len(), 2);
    }

    #[test]
    fn archive_missing_flags_absent_projects_only() {
        let conn = db();
        upsert(&conn, &sample(Platform::Modrinth, "keep", "Keep")).unwrap();
        upsert(&conn, &sample(Platform::Modrinth, "gone", "Gone")).unwrap();
        let n = archive_missing(&conn, Platform::Modrinth, &["keep".into()], "2026-08-11T00:00:00Z").unwrap();
        assert_eq!(n, 1);
        let rows = list(&conn).unwrap();
        let gone = rows.iter().find(|r| r.ext_id == "gone").unwrap();
        let keep = rows.iter().find(|r| r.ext_id == "keep").unwrap();
        assert!(gone.archived_at.is_some());
        assert!(keep.archived_at.is_none());
    }

    #[test]
    fn upsert_link_replaces_automatic_but_never_manual() {
        let conn = db();
        let m = upsert(&conn, &sample(Platform::Modrinth, "m1", "A")).unwrap();
        let c = upsert(&conn, &sample(Platform::CurseForge, "c1", "A")).unwrap();
        upsert_link(&conn, m, c, 0.9, false).unwrap();
        upsert_link(&conn, m, c, 1.0, false).unwrap();
        assert_eq!(links(&conn).unwrap()[0].confidence, 1.0);
        upsert_link(&conn, m, c, 1.0, true).unwrap();
        upsert_link(&conn, m, c, 0.5, false).unwrap();
        let l = &links(&conn).unwrap()[0];
        assert!(l.manual);
        assert_eq!(l.confidence, 1.0);
    }

    #[test]
    fn clear_automatic_links_keeps_manual_ones() {
        let conn = db();
        let m = upsert(&conn, &sample(Platform::Modrinth, "m1", "A")).unwrap();
        let c1 = upsert(&conn, &sample(Platform::CurseForge, "c1", "A")).unwrap();
        let m2 = upsert(&conn, &sample(Platform::Modrinth, "m2", "B")).unwrap();
        let c2 = upsert(&conn, &sample(Platform::CurseForge, "c2", "B")).unwrap();
        upsert_link(&conn, m, c1, 0.9, false).unwrap();
        upsert_link(&conn, m2, c2, 1.0, true).unwrap();
        clear_automatic_links(&conn).unwrap();
        let rows = links(&conn).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].manual);
    }
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml projects
```

Attendu : `cannot find function upsert`.

- [ ] **Step 3 : écrire l'implémentation au-dessus du bloc de test**

```rust
use crate::error::Result;
use crate::models::Platform;
use rusqlite::{params, Connection};

#[derive(Debug, Clone)]
pub struct ProjectUpsert {
    pub platform: Platform,
    pub ext_id: String,
    pub slug: Option<String>,
    pub title: String,
    pub project_type: Option<String>,
    pub url: Option<String>,
    pub icon_url: Option<String>,
    pub created_at: Option<String>,
    pub total_downloads: i64,
    pub followers: i64,
}

#[derive(Debug, Clone)]
pub struct ProjectRow {
    pub id: i64,
    pub platform: Platform,
    pub ext_id: String,
    pub slug: Option<String>,
    pub title: String,
    pub project_type: Option<String>,
    pub url: Option<String>,
    pub icon_url: Option<String>,
    pub created_at: Option<String>,
    pub total_downloads: i64,
    pub followers: i64,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LinkRow {
    pub modrinth_project_id: i64,
    pub cf_project_id: i64,
    pub confidence: f64,
    pub manual: bool,
}

pub fn upsert(conn: &Connection, p: &ProjectUpsert) -> Result<i64> {
    conn.execute(
        "INSERT INTO projects
           (platform, ext_id, slug, title, project_type, url, icon_url, created_at, total_downloads, followers, archived_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL)
         ON CONFLICT(platform, ext_id) DO UPDATE SET
           slug = excluded.slug,
           title = excluded.title,
           project_type = excluded.project_type,
           url = COALESCE(excluded.url, projects.url),
           icon_url = COALESCE(excluded.icon_url, projects.icon_url),
           created_at = COALESCE(excluded.created_at, projects.created_at),
           total_downloads = excluded.total_downloads,
           followers = excluded.followers,
           archived_at = NULL",
        params![
            p.platform.as_str(), p.ext_id, p.slug, p.title, p.project_type,
            p.url, p.icon_url, p.created_at, p.total_downloads, p.followers
        ],
    )?;
    Ok(conn.query_row(
        "SELECT id FROM projects WHERE platform = ?1 AND ext_id = ?2",
        params![p.platform.as_str(), p.ext_id],
        |r| r.get(0),
    )?)
}

fn row_to_project(r: &rusqlite::Row) -> rusqlite::Result<ProjectRow> {
    Ok(ProjectRow {
        id: r.get(0)?,
        platform: Platform::from_str_lossy(&r.get::<_, String>(1)?),
        ext_id: r.get(2)?,
        slug: r.get(3)?,
        title: r.get(4)?,
        project_type: r.get(5)?,
        url: r.get(6)?,
        icon_url: r.get(7)?,
        created_at: r.get(8)?,
        total_downloads: r.get(9)?,
        followers: r.get(10)?,
        archived_at: r.get(11)?,
    })
}

const SELECT_PROJECT: &str = "SELECT id, platform, ext_id, slug, title, project_type, url, icon_url, created_at, total_downloads, followers, archived_at FROM projects";

pub fn list(conn: &Connection) -> Result<Vec<ProjectRow>> {
    let mut stmt = conn.prepare(&format!("{SELECT_PROJECT} ORDER BY title COLLATE NOCASE"))?;
    let rows = stmt.query_map([], row_to_project)?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn list_by_platform(conn: &Connection, platform: Platform) -> Result<Vec<ProjectRow>> {
    let mut stmt = conn.prepare(&format!("{SELECT_PROJECT} WHERE platform = ?1 ORDER BY title COLLATE NOCASE"))?;
    let rows = stmt
        .query_map(params![platform.as_str()], row_to_project)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn archive_missing(
    conn: &Connection,
    platform: Platform,
    seen_ext_ids: &[String],
    now: &str,
) -> Result<usize> {
    let placeholders = if seen_ext_ids.is_empty() {
        "''".to_string()
    } else {
        seen_ext_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",")
    };
    let sql = format!(
        "UPDATE projects SET archived_at = ?1
         WHERE platform = ?2 AND archived_at IS NULL AND ext_id NOT IN ({placeholders})"
    );
    let mut values: Vec<Box<dyn rusqlite::ToSql>> =
        vec![Box::new(now.to_string()), Box::new(platform.as_str().to_string())];
    for id in seen_ext_ids {
        values.push(Box::new(id.clone()));
    }
    let refs: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    Ok(conn.execute(&sql, refs.as_slice())?)
}

pub fn upsert_link(
    conn: &Connection,
    modrinth_id: i64,
    cf_id: i64,
    confidence: f64,
    manual: bool,
) -> Result<()> {
    conn.execute(
        "INSERT INTO links (modrinth_project_id, cf_project_id, confidence, manual)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(modrinth_project_id, cf_project_id) DO UPDATE SET
           confidence = CASE WHEN links.manual = 1 THEN links.confidence ELSE excluded.confidence END,
           manual = CASE WHEN links.manual = 1 THEN 1 ELSE excluded.manual END",
        params![modrinth_id, cf_id, confidence, manual as i64],
    )?;
    Ok(())
}

pub fn links(conn: &Connection) -> Result<Vec<LinkRow>> {
    let mut stmt = conn.prepare(
        "SELECT modrinth_project_id, cf_project_id, confidence, manual FROM links",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(LinkRow {
                modrinth_project_id: r.get(0)?,
                cf_project_id: r.get(1)?,
                confidence: r.get(2)?,
                manual: r.get::<_, i64>(3)? == 1,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn clear_automatic_links(conn: &Connection) -> Result<usize> {
    Ok(conn.execute("DELETE FROM links WHERE manual = 0", [])?)
}

pub fn delete_link(conn: &Connection, modrinth_id: i64, cf_id: i64) -> Result<usize> {
    Ok(conn.execute(
        "DELETE FROM links WHERE modrinth_project_id = ?1 AND cf_project_id = ?2",
        params![modrinth_id, cf_id],
    )?)
}
```

- [ ] **Step 4 : déclarer le sous-module**

Dans `src-tauri/src/store/mod.rs`, ajoute `pub mod projects;` sous `pub mod schema;`.

- [ ] **Step 5 : lancer les tests**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml projects
```

Attendu : `test result: ok. 5 passed`.

- [ ] **Step 6 : commit**

```powershell
git add -A
git commit -m "feat: persistance des projets et des liens inter-plateformes"
```

---

### Task 7 : Séries, pays, snapshots, versions, évènements

**Files:**
- Create: `src-tauri/src/store/metrics.rs`
- Modify: `src-tauri/src/store/mod.rs`

Point critique : les revenus sont stockés en chaîne décimale exacte, jamais en flottant. Le delta CurseForge se calcule entre snapshots consécutifs et n'est jamais négatif — CFWidget peut renvoyer un total corrigé à la baisse, auquel cas le delta du jour vaut zéro.

- [ ] **Step 1 : écrire les tests qui échouent**

Dans `src-tauri/src/store/metrics.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Platform;
    use crate::store::projects::{upsert, ProjectUpsert};
    use crate::store::schema::migrate;
    use rusqlite::Connection;

    fn db() -> (Connection, i64) {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        let id = upsert(&conn, &ProjectUpsert {
            platform: Platform::Modrinth,
            ext_id: "abc".into(),
            slug: Some("abc".into()),
            title: "ABC".into(),
            project_type: Some("mod".into()),
            url: None, icon_url: None, created_at: None,
            total_downloads: 0, followers: 0,
        }).unwrap();
        (conn, id)
    }

    #[test]
    fn upsert_daily_is_idempotent_and_updates() {
        let (conn, id) = db();
        upsert_daily(&conn, id, "2026-08-01", Some(10), Some(3), None).unwrap();
        upsert_daily(&conn, id, "2026-08-01", Some(12), None, Some("0.5")).unwrap();
        let rows = daily_range(&conn, "2026-08-01", "2026-08-02").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].downloads, Some(12));
        assert_eq!(rows[0].views, Some(3), "une valeur absente ne doit pas effacer l'existante");
        assert_eq!(rows[0].revenue.as_deref(), Some("0.5"));
    }

    #[test]
    fn last_day_returns_none_on_empty_then_max_day() {
        let (conn, id) = db();
        assert_eq!(last_metrics_day(&conn).unwrap(), None);
        upsert_daily(&conn, id, "2026-07-30", Some(1), None, None).unwrap();
        upsert_daily(&conn, id, "2026-08-02", Some(1), None, None).unwrap();
        assert_eq!(last_metrics_day(&conn).unwrap().as_deref(), Some("2026-08-02"));
    }

    #[test]
    fn snapshot_deltas_never_go_negative() {
        let (conn, id) = db();
        insert_snapshot(&conn, id, "2026-08-01T00:00:00Z", 100, Some(10)).unwrap();
        insert_snapshot(&conn, id, "2026-08-02T00:00:00Z", 150, Some(10)).unwrap();
        insert_snapshot(&conn, id, "2026-08-03T00:00:00Z", 140, Some(10)).unwrap();
        let d = snapshot_deltas(&conn).unwrap();
        assert_eq!(d.get(&(id, "2026-08-02".to_string())).copied(), Some(50));
        assert_eq!(d.get(&(id, "2026-08-03".to_string())).copied(), Some(0));
        assert_eq!(d.get(&(id, "2026-08-01".to_string())), None, "le premier snapshot n'a pas de delta");
    }

    #[test]
    fn events_are_deduplicated() {
        let (conn, id) = db();
        insert_event(&conn, "modrinth", "2026-08-01T00:00:00Z", "status_change", Some(id), "ABC", "approuve").unwrap();
        insert_event(&conn, "modrinth", "2026-08-01T00:00:00Z", "status_change", Some(id), "ABC", "approuve").unwrap();
        assert_eq!(recent_events(&conn, 10).unwrap().len(), 1);
    }

    #[test]
    fn meta_reads_back_what_was_written() {
        let (conn, _) = db();
        assert_eq!(get_meta(&conn, "balance").unwrap(), None);
        set_meta(&conn, "balance", "12.34").unwrap();
        set_meta(&conn, "balance", "56.78").unwrap();
        assert_eq!(get_meta(&conn, "balance").unwrap().as_deref(), Some("56.78"));
    }

    #[test]
    fn sync_run_records_start_then_finish() {
        let (conn, _) = db();
        let run = start_sync_run(&conn, "modrinth", "2026-08-11T10:00:00Z").unwrap();
        finish_sync_run(&conn, run, "2026-08-11T10:00:05Z", "ok", "3 projets").unwrap();
        let f = freshness(&conn).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].status, "ok");
        assert_eq!(f[0].finished_at.as_deref(), Some("2026-08-11T10:00:05Z"));
    }
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml metrics
```

Attendu : `cannot find function upsert_daily`.

- [ ] **Step 3 : écrire l'implémentation au-dessus du bloc de test**

```rust
use crate::error::Result;
use crate::models::Freshness;
use rusqlite::{params, Connection};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DailyRow {
    pub project_id: i64,
    pub day: String,
    pub downloads: Option<i64>,
    pub views: Option<i64>,
    pub revenue: Option<String>,
}

pub fn upsert_daily(
    conn: &Connection,
    project_id: i64,
    day: &str,
    downloads: Option<i64>,
    views: Option<i64>,
    revenue: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO metrics_daily (project_id, day, downloads, views, revenue)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(project_id, day) DO UPDATE SET
           downloads = COALESCE(excluded.downloads, metrics_daily.downloads),
           views = COALESCE(excluded.views, metrics_daily.views),
           revenue = COALESCE(excluded.revenue, metrics_daily.revenue)",
        params![project_id, day, downloads, views, revenue],
    )?;
    Ok(())
}

pub fn daily_range(conn: &Connection, from: &str, to: &str) -> Result<Vec<DailyRow>> {
    let mut stmt = conn.prepare(
        "SELECT project_id, day, downloads, views, revenue
         FROM metrics_daily WHERE day >= ?1 AND day < ?2 ORDER BY day",
    )?;
    let rows = stmt
        .query_map(params![from, to], |r| {
            Ok(DailyRow {
                project_id: r.get(0)?,
                day: r.get(1)?,
                downloads: r.get(2)?,
                views: r.get(3)?,
                revenue: r.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn last_metrics_day(conn: &Connection) -> Result<Option<String>> {
    Ok(conn.query_row("SELECT MAX(day) FROM metrics_daily", [], |r| r.get(0))?)
}

pub fn upsert_country(
    conn: &Connection,
    project_id: i64,
    day: &str,
    country: &str,
    downloads: i64,
) -> Result<()> {
    conn.execute(
        "INSERT INTO countries_daily (project_id, day, country, downloads)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(project_id, day, country) DO UPDATE SET downloads = excluded.downloads",
        params![project_id, day, country, downloads],
    )?;
    Ok(())
}

pub fn insert_snapshot(
    conn: &Connection,
    project_id: i64,
    taken_at: &str,
    total: i64,
    monthly: Option<i64>,
) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO cf_snapshots (project_id, taken_at, total_downloads, monthly_downloads)
         VALUES (?1, ?2, ?3, ?4)",
        params![project_id, taken_at, total, monthly],
    )?;
    Ok(())
}

/// Delta quotidien par projet CurseForge, borné à zéro.
/// La clé est (project_id, jour du snapshot courant).
pub fn snapshot_deltas(conn: &Connection) -> Result<HashMap<(i64, String), i64>> {
    let mut stmt = conn.prepare(
        "SELECT project_id, substr(taken_at, 1, 10) AS day, MAX(total_downloads)
         FROM cf_snapshots GROUP BY project_id, day ORDER BY project_id, day",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut out = HashMap::new();
    let mut previous: Option<(i64, i64)> = None;
    for (project_id, day, total) in rows {
        if let Some((prev_project, prev_total)) = previous {
            if prev_project == project_id {
                out.insert((project_id, day.clone()), (total - prev_total).max(0));
            }
        }
        previous = Some((project_id, total));
    }
    Ok(out)
}

pub fn snapshot_day_count(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT COUNT(DISTINCT substr(taken_at, 1, 10)) FROM cf_snapshots",
        [],
        |r| r.get(0),
    )?)
}

pub fn upsert_version(
    conn: &Connection,
    project_id: i64,
    version_id: &str,
    version_number: Option<&str>,
    game_versions: &[String],
    loaders: &[String],
    downloads: i64,
    date_published: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO versions (project_id, version_id, version_number, game_versions, loaders, downloads, date_published)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(project_id, version_id) DO UPDATE SET
           version_number = excluded.version_number,
           game_versions = excluded.game_versions,
           loaders = excluded.loaders,
           downloads = excluded.downloads,
           date_published = excluded.date_published",
        params![
            project_id,
            version_id,
            version_number,
            serde_json::to_string(game_versions)?,
            serde_json::to_string(loaders)?,
            downloads,
            date_published
        ],
    )?;
    Ok(())
}

pub fn insert_event(
    conn: &Connection,
    source: &str,
    occurred_at: &str,
    kind: &str,
    project_id: Option<i64>,
    title: &str,
    detail: &str,
) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO events (source, occurred_at, kind, project_id, title, detail)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![source, occurred_at, kind, project_id, title, detail],
    )?;
    Ok(())
}

pub fn recent_events(conn: &Connection, limit: i64) -> Result<Vec<crate::models::EventRow>> {
    let mut stmt = conn.prepare(
        "SELECT occurred_at, kind, title, detail FROM events ORDER BY occurred_at DESC LIMIT ?1",
    )?;
    let rows = stmt
        .query_map(params![limit], |r| {
            Ok(crate::models::EventRow {
                occurred_at: r.get(0)?,
                kind: r.get(1)?,
                title: r.get(2)?,
                detail: r.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn set_meta(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn get_meta(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM meta WHERE key = ?1")?;
    let mut rows = stmt.query(params![key])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub fn start_sync_run(conn: &Connection, provider: &str, started_at: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO sync_runs (started_at, provider, status) VALUES (?1, ?2, 'running')",
        params![started_at, provider],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn finish_sync_run(
    conn: &Connection,
    id: i64,
    finished_at: &str,
    status: &str,
    detail: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE sync_runs SET finished_at = ?1, status = ?2, detail = ?3 WHERE id = ?4",
        params![finished_at, status, detail, id],
    )?;
    Ok(())
}

/// Dernier cycle terminé par provider.
pub fn freshness(conn: &Connection) -> Result<Vec<Freshness>> {
    let mut stmt = conn.prepare(
        "SELECT provider, status, finished_at, detail FROM sync_runs r
         WHERE r.id = (SELECT MAX(id) FROM sync_runs WHERE provider = r.provider)
         ORDER BY provider",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(Freshness {
                provider: r.get(0)?,
                status: r.get(1)?,
                finished_at: r.get(2)?,
                detail: r.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}
```

- [ ] **Step 4 : déclarer le sous-module**

Dans `src-tauri/src/store/mod.rs`, ajoute `pub mod metrics;`.

- [ ] **Step 5 : lancer les tests**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml metrics
```

Attendu : `test result: ok. 6 passed`.

- [ ] **Step 6 : commit**

```powershell
git add -A
git commit -m "feat: persistance des series, snapshots, versions et evenements"
```

---

### Task 8 : Politique réseau commune

**Files:**
- Create: `src-tauri/src/providers/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1 : écrire les tests qui échouent**

Dans `src-tauri/src/providers/mod.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_and_is_bounded() {
        assert_eq!(backoff_ms(0), 400);
        assert_eq!(backoff_ms(1), 800);
        assert_eq!(backoff_ms(2), 1600);
        assert_eq!(backoff_ms(9), MAX_BACKOFF_MS);
    }

    #[test]
    fn only_transient_statuses_are_retried() {
        assert!(should_retry(429));
        assert!(should_retry(500));
        assert!(should_retry(503));
        assert!(!should_retry(400));
        assert!(!should_retry(401));
        assert!(!should_retry(404));
        assert!(!should_retry(200));
    }

    #[test]
    fn chunking_respects_batch_size() {
        let ids: Vec<String> = (0..25).map(|i| i.to_string()).collect();
        let chunks = chunk_ids(&ids, 10);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].len(), 10);
        assert_eq!(chunks[2].len(), 5);
    }
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml providers
```

Attendu : `cannot find function backoff_ms`.

- [ ] **Step 3 : écrire l'implémentation au-dessus du bloc de test**

```rust
pub mod curseforge;
pub mod modrinth;

use crate::error::{AppError, Result};

pub const USER_AGENT: &str = concat!("Dreyka-Oas/chartographer/", env!("CARGO_PKG_VERSION"));
pub const MAX_RETRIES: u32 = 3;
pub const MAX_BACKOFF_MS: u64 = 8_000;
/// Modrinth accepte 300 requêtes par minute ; on reste large sous le plafond.
pub const ANALYTICS_BATCH: usize = 10;

pub fn backoff_ms(attempt: u32) -> u64 {
    (400u64 << attempt.min(16)).min(MAX_BACKOFF_MS)
}

pub fn should_retry(status: u16) -> bool {
    status == 429 || (500..600).contains(&status)
}

pub fn chunk_ids(ids: &[String], size: usize) -> Vec<Vec<String>> {
    ids.chunks(size.max(1)).map(|c| c.to_vec()).collect()
}

pub fn http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| AppError::Remote { provider: "http".into(), detail: e.to_string() })
}

/// Exécute une requête avec retry borné sur 429 et 5xx.
/// `make` est rappelé à chaque tentative car une RequestBuilder n'est pas clonable.
pub async fn send_with_retry(
    provider: &str,
    make: impl Fn() -> reqwest::RequestBuilder,
) -> Result<reqwest::Response> {
    let mut attempt = 0;
    loop {
        let result = make().send().await;
        match result {
            Ok(response) => {
                let status = response.status().as_u16();
                if status == 401 || status == 403 {
                    return Err(AppError::auth(provider, format!("HTTP {status}")));
                }
                if !should_retry(status) {
                    return Ok(response);
                }
                if attempt >= MAX_RETRIES {
                    return Err(AppError::remote(provider, format!("HTTP {status} après {attempt} reprises")));
                }
            }
            Err(e) => {
                if attempt >= MAX_RETRIES {
                    return Err(AppError::remote(provider, e.to_string()));
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(backoff_ms(attempt))).await;
        attempt += 1;
    }
}
```

- [ ] **Step 4 : déclarer le module et créer les fichiers vides des providers**

Dans `src-tauri/src/lib.rs`, ajoute `pub mod providers;`. Crée `src-tauri/src/providers/modrinth.rs` et `src-tauri/src/providers/curseforge.rs` vides pour que la compilation passe.

- [ ] **Step 5 : lancer les tests**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml providers
```

Attendu : `test result: ok. 3 passed`.

- [ ] **Step 6 : commit**

```powershell
git add -A
git commit -m "feat: politique reseau commune, retry borne et lots d'analyse"
```

---

### Task 9 : Client Modrinth

**Files:**
- Modify: `src-tauri/src/providers/modrinth.rs`

Formes de réponse vérifiées en conditions réelles :

- `/v2/user` → objet avec `id`, `username`, `payout_data.balance` (nombre).
- `/v2/user/{id}/projects` → tableau d'objets avec `id`, `slug`, `title`, `project_type`, `downloads`, `followers`, `icon_url`, `published`.
- `/v3/analytics/downloads` → `{ "<project_id>": { "<timestamp_unix_secondes>": <entier> } }`. Les timestamps sont des **clés de chaîne**.
- `/v3/analytics/revenue` → même forme, valeurs en **chaînes décimales de haute précision** comme `"0.00762273691987854525"`.
- `/v3/analytics/countries/downloads` → `{ "<project_id>": { "<ISO2>": <entier> } }`, avec deux clés spéciales : `"XX"` (inconnu) et `""` (non renseigné).
- `/v2/user/{id}/notifications` → tableau avec `id`, `read`, `created`, `body.type`, `body.project_id`, et selon le type `body.old_status` / `body.new_status`.

L'en-tête d'authentification est `Authorization: <token>`, **sans** préfixe `Bearer`.

- [ ] **Step 1 : écrire les tests qui échouent**

Dans `src-tauri/src/providers/modrinth.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_series_reads_string_timestamps() {
        let raw = r#"{"6P28kqbu":{"1784073600":838,"1783641600":863},"W0JWCVNo":{"1784073600":14}}"#;
        let out = parse_series(raw).unwrap();
        assert_eq!(out["6P28kqbu"][&1_783_641_600], 863);
        assert_eq!(out["6P28kqbu"][&1_784_073_600], 838);
        assert_eq!(out["W0JWCVNo"].len(), 1);
    }

    #[test]
    fn parse_revenue_keeps_full_decimal_precision() {
        let raw = r#"{"W0JWCVNo":{"1785888000":"0.00762273691987854525"}}"#;
        let out = parse_revenue(raw).unwrap();
        let value = out["W0JWCVNo"][&1_785_888_000];
        assert_eq!(value.to_string(), "0.00762273691987854525");
    }

    #[test]
    fn parse_countries_keeps_special_keys_apart() {
        let raw = r#"{"W0JWCVNo":{"DE":88,"XX":558,"":454,"US":456}}"#;
        let out = parse_countries(raw).unwrap();
        assert_eq!(out["W0JWCVNo"]["DE"], 88);
        assert_eq!(out["W0JWCVNo"]["XX"], 558);
        assert_eq!(out["W0JWCVNo"][""], 454);
        assert_eq!(out["W0JWCVNo"].len(), 4);
    }

    #[test]
    fn timestamp_to_day_is_utc() {
        assert_eq!(timestamp_to_day(1_784_073_600), "2026-07-06");
    }

    #[test]
    fn parse_projects_maps_all_fields() {
        let raw = r#"[{"id":"6P28kqbu","slug":"vein-vantage","title":"Vein Vantage",
            "project_type":"mod","downloads":176968,"followers":6,
            "icon_url":"https://cdn.modrinth.com/x.png","published":"2024-06-01T10:00:00Z"}]"#;
        let out = parse_projects(raw).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].slug, "vein-vantage");
        assert_eq!(out[0].downloads, 176_968);
        assert_eq!(out[0].followers, 6);
    }

    #[test]
    fn parse_notifications_renders_status_change() {
        let raw = r#"[{"id":"OT8","read":false,"created":"2026-03-15T21:51:39.314925Z",
            "body":{"type":"status_change","project_id":"YCu7AAOD",
                    "old_status":"processing","new_status":"approved"}}]"#;
        let out = parse_notifications(raw).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, "status_change");
        assert_eq!(out[0].project_ext_id.as_deref(), Some("YCu7AAOD"));
        assert!(out[0].detail.contains("processing"));
        assert!(out[0].detail.contains("approved"));
    }
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml modrinth
```

Attendu : `cannot find function parse_series`.

- [ ] **Step 3 : écrire l'implémentation au-dessus du bloc de test**

```rust
use crate::error::{AppError, Result};
use crate::providers::{http_client, send_with_retry, ANALYTICS_BATCH};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;

const BASE: &str = "https://api.modrinth.com";
const PROVIDER: &str = "modrinth";

pub type SeriesMap = HashMap<String, BTreeMap<i64, i64>>;
pub type RevenueMap = HashMap<String, BTreeMap<i64, Decimal>>;
pub type CountryMap = HashMap<String, HashMap<String, i64>>;

#[derive(Debug, Clone, Deserialize)]
pub struct ModrinthUser {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub payout_data: PayoutData,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PayoutData {
    #[serde(default)]
    pub balance: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModrinthProject {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub project_type: Option<String>,
    #[serde(default)]
    pub downloads: i64,
    #[serde(default)]
    pub followers: i64,
    pub icon_url: Option<String>,
    pub published: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModrinthVersion {
    pub id: String,
    pub version_number: Option<String>,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    #[serde(default)]
    pub downloads: i64,
    pub date_published: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub occurred_at: String,
    pub kind: String,
    pub project_ext_id: Option<String>,
    pub detail: String,
}

pub fn timestamp_to_day(ts: i64) -> String {
    DateTime::<Utc>::from_timestamp(ts, 0)
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

pub fn parse_series(raw: &str) -> Result<SeriesMap> {
    let parsed: HashMap<String, HashMap<String, i64>> = serde_json::from_str(raw)?;
    Ok(parsed
        .into_iter()
        .map(|(project, points)| {
            let series = points
                .into_iter()
                .filter_map(|(ts, value)| ts.parse::<i64>().ok().map(|t| (t, value)))
                .collect();
            (project, series)
        })
        .collect())
}

pub fn parse_revenue(raw: &str) -> Result<RevenueMap> {
    let parsed: HashMap<String, HashMap<String, String>> = serde_json::from_str(raw)?;
    Ok(parsed
        .into_iter()
        .map(|(project, points)| {
            let series = points
                .into_iter()
                .filter_map(|(ts, value)| {
                    let ts = ts.parse::<i64>().ok()?;
                    let amount = Decimal::from_str(&value).ok()?;
                    Some((ts, amount))
                })
                .collect();
            (project, series)
        })
        .collect())
}

pub fn parse_countries(raw: &str) -> Result<CountryMap> {
    Ok(serde_json::from_str(raw)?)
}

pub fn parse_projects(raw: &str) -> Result<Vec<ModrinthProject>> {
    Ok(serde_json::from_str(raw)?)
}

pub fn parse_versions(raw: &str) -> Result<Vec<ModrinthVersion>> {
    Ok(serde_json::from_str(raw)?)
}

pub fn parse_notifications(raw: &str) -> Result<Vec<Notification>> {
    #[derive(Deserialize)]
    struct Raw {
        created: String,
        body: serde_json::Value,
    }
    let rows: Vec<Raw> = serde_json::from_str(raw)?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let kind = r.body.get("type").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
            let project_ext_id = r
                .body
                .get("project_id")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let detail = match (r.body.get("old_status"), r.body.get("new_status")) {
                (Some(old), Some(new)) => format!(
                    "{} vers {}",
                    old.as_str().unwrap_or_default(),
                    new.as_str().unwrap_or_default()
                ),
                _ => r.body.to_string(),
            };
            Notification { occurred_at: r.created, kind, project_ext_id, detail }
        })
        .collect())
}

pub struct ModrinthClient {
    http: reqwest::Client,
    token: String,
}

impl ModrinthClient {
    pub fn new(token: &str) -> Result<Self> {
        Ok(Self { http: http_client()?, token: token.to_string() })
    }

    async fn get_text(&self, url: &str) -> Result<String> {
        let response = send_with_retry(PROVIDER, || {
            self.http.get(url).header("Authorization", &self.token)
        })
        .await?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| AppError::remote(PROVIDER, e.to_string()))?;
        if !status.is_success() {
            return Err(AppError::remote(PROVIDER, format!("HTTP {status} sur {url}")));
        }
        Ok(body)
    }

    pub async fn me(&self) -> Result<ModrinthUser> {
        let body = self.get_text(&format!("{BASE}/v2/user")).await?;
        Ok(serde_json::from_str(&body)?)
    }

    pub async fn projects(&self, user_id: &str) -> Result<Vec<ModrinthProject>> {
        let body = self.get_text(&format!("{BASE}/v2/user/{user_id}/projects")).await?;
        parse_projects(&body)
    }

    pub async fn versions(&self, project_id: &str) -> Result<Vec<ModrinthVersion>> {
        let body = self.get_text(&format!("{BASE}/v2/project/{project_id}/version")).await?;
        parse_versions(&body)
    }

    pub async fn notifications(&self, user_id: &str) -> Result<Vec<Notification>> {
        let body = self.get_text(&format!("{BASE}/v2/user/{user_id}/notifications")).await?;
        parse_notifications(&body)
    }

    fn analytics_url(&self, path: &str, ids: &[String], start: DateTime<Utc>, end: DateTime<Utc>) -> String {
        let ids_json = serde_json::to_string(ids).unwrap_or_else(|_| "[]".into());
        format!(
            "{BASE}/v3/analytics/{path}?project_ids={}&start_date={}&end_date={}&resolution_minutes=1440",
            urlencode(&ids_json),
            urlencode(&start.to_rfc3339()),
            urlencode(&end.to_rfc3339())
        )
    }

    pub async fn analytics_downloads(&self, ids: &[String], start: DateTime<Utc>, end: DateTime<Utc>) -> Result<SeriesMap> {
        let mut merged = SeriesMap::new();
        for batch in crate::providers::chunk_ids(ids, ANALYTICS_BATCH) {
            let body = self.get_text(&self.analytics_url("downloads", &batch, start, end)).await?;
            merged.extend(parse_series(&body)?);
        }
        Ok(merged)
    }

    pub async fn analytics_views(&self, ids: &[String], start: DateTime<Utc>, end: DateTime<Utc>) -> Result<SeriesMap> {
        let mut merged = SeriesMap::new();
        for batch in crate::providers::chunk_ids(ids, ANALYTICS_BATCH) {
            let body = self.get_text(&self.analytics_url("views", &batch, start, end)).await?;
            merged.extend(parse_series(&body)?);
        }
        Ok(merged)
    }

    pub async fn analytics_revenue(&self, ids: &[String], start: DateTime<Utc>, end: DateTime<Utc>) -> Result<RevenueMap> {
        let mut merged = RevenueMap::new();
        for batch in crate::providers::chunk_ids(ids, ANALYTICS_BATCH) {
            let body = self.get_text(&self.analytics_url("revenue", &batch, start, end)).await?;
            merged.extend(parse_revenue(&body)?);
        }
        Ok(merged)
    }

    pub async fn analytics_countries(&self, ids: &[String], start: DateTime<Utc>, end: DateTime<Utc>) -> Result<CountryMap> {
        let mut merged = CountryMap::new();
        for batch in crate::providers::chunk_ids(ids, ANALYTICS_BATCH) {
            let body = self
                .get_text(&self.analytics_url("countries/downloads", &batch, start, end))
                .await?;
            merged.extend(parse_countries(&body)?);
        }
        Ok(merged)
    }
}

fn urlencode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}
```

- [ ] **Step 4 : lancer les tests**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml modrinth
```

Attendu : `test result: ok. 6 passed`.

- [ ] **Step 5 : commit**

```powershell
git add -A
git commit -m "feat: client Modrinth v2 et v3 avec parsing teste"
```

---

### Task 10 : Client CurseForge

**Files:**
- Modify: `src-tauri/src/providers/curseforge.rs`

Aucune authentification. CFWidget est public et répond `202` quand la ressource n'est pas encore en cache.

- [ ] **Step 1 : écrire les tests qui échouent**

Dans `src-tauri/src/providers/curseforge.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_author_lists_projects() {
        let raw = r#"{"id":108432004,"username":"DreykaOas_official",
            "projects":[{"id":1002185,"name":"Mobs Blocker"},{"id":1412622,"name":"Extended Time"}]}"#;
        let out = parse_author(raw).unwrap();
        assert_eq!(out.username, "DreykaOas_official");
        assert_eq!(out.projects.len(), 2);
        assert_eq!(out.projects[0].id, 1_002_185);
    }

    #[test]
    fn parse_project_reads_downloads_and_url() {
        let raw = r#"{"id":1002185,"title":"Mobs Blocker","type":"Mods",
            "urls":{"curseforge":"https://www.curseforge.com/minecraft/mc-mods/mobblocker",
                    "project":"https://www.curseforge.com/minecraft/mc-mods/mobblocker"},
            "downloads":{"monthly":0,"total":86753},
            "thumbnail":"https://media.forgecdn.net/x.png",
            "created_at":"2024-04-13T11:39:21.023Z"}"#;
        let out = parse_project(raw).unwrap();
        assert_eq!(out.downloads_total, 86_753);
        assert_eq!(out.downloads_monthly, 0);
        assert_eq!(out.slug.as_deref(), Some("mobblocker"));
        assert_eq!(out.title, "Mobs Blocker");
    }

    #[test]
    fn slug_from_url_handles_missing_and_trailing_slash() {
        assert_eq!(slug_from_url("https://www.curseforge.com/minecraft/mc-mods/zone-cleaner"), Some("zone-cleaner".into()));
        assert_eq!(slug_from_url("https://www.curseforge.com/minecraft/mc-mods/zone-cleaner/"), Some("zone-cleaner".into()));
        assert_eq!(slug_from_url(""), None);
    }

}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml curseforge
```

Attendu : `cannot find function parse_author`.

- [ ] **Step 3 : écrire l'implémentation au-dessus du bloc de test**

```rust
use crate::error::{AppError, Result};
use crate::providers::{http_client, send_with_retry};
use serde::Deserialize;

const WIDGET: &str = "https://api.cfwidget.com";
const PROVIDER: &str = "curseforge";

#[derive(Debug, Clone, Deserialize)]
pub struct CfAuthorProject {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CfAuthor {
    pub id: i64,
    pub username: String,
    #[serde(default)]
    pub projects: Vec<CfAuthorProject>,
}

#[derive(Debug, Clone)]
pub struct CfProject {
    pub id: i64,
    pub title: String,
    pub slug: Option<String>,
    pub project_type: Option<String>,
    pub url: Option<String>,
    pub thumbnail: Option<String>,
    pub created_at: Option<String>,
    pub downloads_total: i64,
    pub downloads_monthly: i64,
}

/// CFWidget répond 202 lorsqu'un rafraîchissement est mis en file d'attente.
#[derive(Debug, Clone)]
pub enum CfFetch {
    Ready(Box<CfProject>),
    Queued,
}

pub fn slug_from_url(url: &str) -> Option<String> {
    let trimmed = url.trim_end_matches('/');
    let last = trimmed.rsplit('/').next()?;
    if last.is_empty() || last.starts_with("http") {
        return None;
    }
    Some(last.to_string())
}

pub fn parse_author(raw: &str) -> Result<CfAuthor> {
    Ok(serde_json::from_str(raw)?)
}

pub fn parse_project(raw: &str) -> Result<CfProject> {
    #[derive(Deserialize)]
    struct Downloads {
        #[serde(default)]
        total: i64,
        #[serde(default)]
        monthly: i64,
    }
    #[derive(Deserialize)]
    struct Urls {
        #[serde(default)]
        curseforge: String,
        #[serde(default)]
        project: String,
    }
    #[derive(Deserialize)]
    struct Raw {
        id: i64,
        title: String,
        #[serde(rename = "type")]
        project_type: Option<String>,
        #[serde(default)]
        urls: Option<Urls>,
        #[serde(default)]
        downloads: Option<Downloads>,
        thumbnail: Option<String>,
        created_at: Option<String>,
    }

    let raw: Raw = serde_json::from_str(raw)?;
    let url = raw
        .urls
        .as_ref()
        .map(|u| if u.curseforge.is_empty() { u.project.clone() } else { u.curseforge.clone() })
        .filter(|u| !u.is_empty());
    let downloads = raw.downloads.unwrap_or(Downloads { total: 0, monthly: 0 });

    Ok(CfProject {
        id: raw.id,
        slug: url.as_deref().and_then(slug_from_url),
        title: raw.title,
        project_type: raw.project_type,
        url,
        thumbnail: raw.thumbnail,
        created_at: raw.created_at,
        downloads_total: downloads.total,
        downloads_monthly: downloads.monthly,
    })
}

pub struct CurseForgeClient {
    http: reqwest::Client,
}

impl CurseForgeClient {
    pub fn new() -> Result<Self> {
        Ok(Self { http: http_client()? })
    }

    pub async fn author(&self, username: &str) -> Result<CfAuthor> {
        let url = format!("{WIDGET}/author/search/{username}");
        let response = send_with_retry(PROVIDER, || self.http.get(&url)).await?;
        if response.status().as_u16() == 404 {
            return Err(AppError::Config(format!("pseudo CurseForge introuvable : {username}")));
        }
        let body = response.text().await.map_err(|e| AppError::remote(PROVIDER, e.to_string()))?;
        parse_author(&body)
    }

    pub async fn project(&self, id: i64) -> Result<CfFetch> {
        let url = format!("{WIDGET}/{id}");
        let response = send_with_retry(PROVIDER, || self.http.get(&url)).await?;
        if response.status().as_u16() == 202 {
            return Ok(CfFetch::Queued);
        }
        let body = response.text().await.map_err(|e| AppError::remote(PROVIDER, e.to_string()))?;
        Ok(CfFetch::Ready(Box::new(parse_project(&body)?)))
    }
}
```

- [ ] **Step 4 : lancer les tests**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml curseforge
```

Attendu : `test result: ok. 3 passed`.

- [ ] **Step 5 : commit**

```powershell
git add -A
git commit -m "feat: client CurseForge public via CFWidget"
```

---

### Task 10 bis : Flux OAuth Modrinth

**Files:**
- Create: `src-tauri/src/oauth.rs`
- Modify: `src-tauri/src/lib.rs`

Flux de code d'autorisation avec redirection en boucle locale. L'application ouvre un écouteur TCP sur `127.0.0.1` sur un port libre, ouvre le navigateur système sur la page de consentement, attend la redirection sur `/callback`, vérifie le `state`, échange le code contre un token, puis arrête l'écouteur.

Comportements vérifiés en conditions réelles : l'endpoint d'autorisation répond `500 { "error": "server_error", "description": "Authentication method was not valid" }` sans cookie de session, ce qui impose le navigateur système. L'endpoint de token exige `grant_type`, `code`, `client_id` et `redirect_uri`, et répond `400 { "error": "invalid_client" }` sur un `client_id` inconnu.

- [ ] **Step 1 : écrire les tests qui échouent**

Dans `src-tauri/src/oauth.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorize_url_contains_every_required_parameter() {
        let url = authorize_url("cid", "http://127.0.0.1:7345/callback", "st4te");
        assert!(url.starts_with("https://api.modrinth.com/_internal/oauth/authorize?"));
        assert!(url.contains("client_id=cid"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("state=st4te"));
        assert!(url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A7345%2Fcallback"));
        assert!(url.contains(&format!("scope={}", urlencode(SCOPES))));
    }

    #[test]
    fn parse_callback_extracts_code_and_state() {
        let query = parse_callback("GET /callback?code=abc123&state=st4te HTTP/1.1").unwrap();
        assert_eq!(query.get("code").map(String::as_str), Some("abc123"));
        assert_eq!(query.get("state").map(String::as_str), Some("st4te"));
    }

    #[test]
    fn parse_callback_decodes_percent_escapes() {
        let query = parse_callback("GET /callback?error=access_denied&description=a%20refuse HTTP/1.1").unwrap();
        assert_eq!(query.get("description").map(String::as_str), Some("a refuse"));
    }

    #[test]
    fn parse_callback_rejects_other_paths() {
        assert!(parse_callback("GET /favicon.ico HTTP/1.1").is_none());
    }

    #[test]
    fn state_values_are_unique_and_long_enough() {
        let a = random_state();
        let b = random_state();
        assert_ne!(a, b);
        assert!(a.len() >= 32);
        assert!(a.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn parse_token_response_reads_access_token() {
        let raw = r#"{"access_token":"mrp_xyz","token_type":"Bearer","expires_in":1209600}"#;
        assert_eq!(parse_token_response(raw).unwrap(), "mrp_xyz");
    }

    #[test]
    fn parse_token_response_surfaces_the_api_error() {
        let raw = r#"{"error":"invalid_client","description":"The provided client id was invalid"}"#;
        let message = parse_token_response(raw).unwrap_err().to_string();
        assert!(message.contains("invalid_client"));
        assert!(message.contains("client id was invalid"));
    }
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml oauth
```

Attendu : `cannot find function authorize_url`.

- [ ] **Step 3 : écrire l'implémentation au-dessus du bloc de test**

```rust
use crate::config::{OauthApp, Session};
use crate::error::{AppError, Result};
use crate::providers::{http_client, USER_AGENT};
use chrono::Utc;
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const AUTHORIZE: &str = "https://api.modrinth.com/_internal/oauth/authorize";
const TOKEN: &str = "https://api.modrinth.com/_internal/oauth/token";
const PROVIDER: &str = "modrinth";

/// Portée demandée : lecture du profil, des projets, des versions, des notifications et des analyses.
pub const SCOPES: &str = "USER_READ USER_READ_EMAIL PROJECT_READ VERSION_READ NOTIFICATION_READ ANALYTICS PAYOUTS_READ";

/// Délai au-delà duquel on abandonne l'attente de la redirection.
pub const CALLBACK_TIMEOUT_SECS: u64 = 300;

const PAGE_OK: &str = "<!doctype html><meta charset=\"utf-8\"><title>Chartographer</title>\
<body style=\"background:#0d1013;color:#e6ebf0;font-family:system-ui;display:grid;place-items:center;height:100vh;margin:0\">\
<div style=\"text-align:center\"><h1>Connexion reussie</h1><p>Tu peux fermer cet onglet et revenir a Chartographer.</p></div>";

const PAGE_KO: &str = "<!doctype html><meta charset=\"utf-8\"><title>Chartographer</title>\
<body style=\"background:#0d1013;color:#e6ebf0;font-family:system-ui;display:grid;place-items:center;height:100vh;margin:0\">\
<div style=\"text-align:center\"><h1>Connexion refusee</h1><p>Retourne dans Chartographer pour reessayer.</p></div>";

pub fn urlencode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn urldecode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        i += 3;
                    }
                    Err(_) => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            other => {
                out.push(other);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Valeur anti-rejeu dérivée de l'horloge et de l'adresse d'une allocation.
/// Pas de dépendance à un générateur aléatoire : l'unicité suffit ici.
pub fn random_state() -> String {
    let seed = Box::new(0u8);
    let entropy = format!(
        "{:x}{:x}{:x}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default(),
        std::ptr::addr_of!(*seed) as usize,
        std::process::id()
    );
    entropy.chars().filter(|c| c.is_ascii_alphanumeric()).cycle().take(40).collect()
}

pub fn authorize_url(client_id: &str, redirect_uri: &str, state: &str) -> String {
    format!(
        "{AUTHORIZE}?client_id={}&redirect_uri={}&scope={}&response_type=code&state={}",
        urlencode(client_id),
        urlencode(redirect_uri),
        urlencode(SCOPES),
        urlencode(state)
    )
}

/// Extrait les paramètres de la première ligne d'une requête HTTP vers `/callback`.
pub fn parse_callback(request_line: &str) -> Option<HashMap<String, String>> {
    let target = request_line.split_whitespace().nth(1)?;
    let (path, query) = target.split_once('?')?;
    if path != "/callback" {
        return None;
    }
    Some(
        query
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .map(|(k, v)| (urldecode(k), urldecode(v)))
            .collect(),
    )
}

pub fn parse_token_response(raw: &str) -> Result<String> {
    let value: serde_json::Value = serde_json::from_str(raw)?;
    if let Some(token) = value.get("access_token").and_then(|v| v.as_str()) {
        return Ok(token.to_string());
    }
    let error = value.get("error").and_then(|v| v.as_str()).unwrap_or("réponse inattendue");
    let description = value.get("description").and_then(|v| v.as_str()).unwrap_or("");
    Err(AppError::auth(PROVIDER, format!("{error} : {description}")))
}

async fn wait_for_callback(listener: TcpListener, expected_state: &str) -> Result<String> {
    loop {
        let (mut socket, _) = listener
            .accept()
            .await
            .map_err(|e| AppError::remote(PROVIDER, format!("écouteur local : {e}")))?;

        let mut buffer = [0u8; 4096];
        let read = socket.read(&mut buffer).await.unwrap_or(0);
        let request = String::from_utf8_lossy(&buffer[..read]);
        let first_line = request.lines().next().unwrap_or_default();

        let Some(params) = parse_callback(first_line) else {
            let _ = socket.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n").await;
            continue;
        };

        let outcome = match (params.get("code"), params.get("state")) {
            (Some(code), Some(state)) if state == expected_state => Ok(code.clone()),
            (_, Some(_)) | (_, None) if params.contains_key("error") => Err(AppError::auth(
                PROVIDER,
                params.get("error").cloned().unwrap_or_else(|| "refus".into()),
            )),
            (Some(_), Some(_)) => Err(AppError::auth(PROVIDER, "paramètre state invalide".into())),
            _ => Err(AppError::auth(PROVIDER, "réponse d'autorisation incomplète".into())),
        };

        let page = if outcome.is_ok() { PAGE_OK } else { PAGE_KO };
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            page.len(),
            page
        );
        let _ = socket.write_all(response.as_bytes()).await;
        let _ = socket.shutdown().await;
        return outcome;
    }
}

async fn exchange_code(app: &OauthApp, code: &str, redirect_uri: &str) -> Result<String> {
    let client = http_client()?;
    let response = client
        .post(TOKEN)
        .header("User-Agent", USER_AGENT)
        .header("Authorization", &app.client_secret)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("client_id", app.client_id.as_str()),
            ("redirect_uri", redirect_uri),
        ])
        .send()
        .await
        .map_err(|e| AppError::remote(PROVIDER, e.to_string()))?;

    let body = response
        .text()
        .await
        .map_err(|e| AppError::remote(PROVIDER, e.to_string()))?;
    parse_token_response(&body)
}

/// Ouvre le navigateur, attend la redirection, échange le code et renvoie la session.
/// `open_browser` est injecté pour que l'appelant décide comment ouvrir l'URL.
pub async fn login(
    app: &OauthApp,
    open_browser: impl FnOnce(&str) -> Result<()>,
) -> Result<Session> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| AppError::remote(PROVIDER, format!("impossible d'ouvrir un port local : {e}")))?;
    let port = listener
        .local_addr()
        .map_err(|e| AppError::remote(PROVIDER, e.to_string()))?
        .port();
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");
    let state = random_state();

    open_browser(&authorize_url(&app.client_id, &redirect_uri, &state))?;

    let code = tokio::time::timeout(
        std::time::Duration::from_secs(CALLBACK_TIMEOUT_SECS),
        wait_for_callback(listener, &state),
    )
    .await
    .map_err(|_| AppError::auth(PROVIDER, "délai d'autorisation dépassé".into()))??;

    let token = exchange_code(app, &code, &redirect_uri).await?;
    let user = crate::providers::modrinth::ModrinthClient::new(&token)?.me().await?;

    Ok(Session {
        token,
        user_id: user.id,
        username: user.username,
        obtained_at: Utc::now().to_rfc3339(),
    })
}
```

- [ ] **Step 4 : déclarer le module**

Dans `src-tauri/src/lib.rs`, ajoute `pub mod oauth;`.

- [ ] **Step 5 : lancer les tests**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml oauth
```

Attendu : `test result: ok. 7 passed`.

- [ ] **Step 6 : commit**

```powershell
git add -A
git commit -m "feat: connexion Modrinth par OAuth avec redirection en boucle locale"
```

---

### Task 11 : Agrégations pour la page de vision

**Files:**
- Create: `src-tauri/src/store/queries.rs`
- Modify: `src-tauri/src/store/mod.rs`

Cette couche transforme les tables en `Overview`. Elle ne fait aucun réseau et se teste entièrement sur base en mémoire.

- [ ] **Step 1 : écrire les tests qui échouent**

Dans `src-tauri/src/store/queries.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::metrics::{insert_snapshot, upsert_country, upsert_daily, upsert_version};
    use crate::store::projects::{upsert, upsert_link, ProjectUpsert};
    use crate::store::schema::migrate;
    use rusqlite::Connection;

    fn seed() -> (Connection, i64, i64) {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        let mk = |platform, ext: &str, title: &str, total| ProjectUpsert {
            platform,
            ext_id: ext.into(),
            slug: Some(title.to_lowercase().replace(' ', "-")),
            title: title.into(),
            project_type: Some("mod".into()),
            url: None, icon_url: None, created_at: None,
            total_downloads: total, followers: 5,
        };
        let m = upsert(&conn, &mk(Platform::Modrinth, "m1", "Mobs Blocker", 23_225)).unwrap();
        let c = upsert(&conn, &mk(Platform::CurseForge, "1002185", "Mobs Blocker", 86_753)).unwrap();
        upsert_link(&conn, m, c, 1.0, false).unwrap();
        (conn, m, c)
    }

    #[test]
    fn timeline_merges_modrinth_series_and_curseforge_deltas() {
        let (conn, m, c) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(40), None, None).unwrap();
        insert_snapshot(&conn, c, "2026-08-09T00:00:00Z", 100, None).unwrap();
        insert_snapshot(&conn, c, "2026-08-10T00:00:00Z", 175, None).unwrap();
        let points = timeline(&conn, "2026-08-01", "2026-08-11").unwrap();
        let day = points.iter().find(|p| p.day == "2026-08-10").unwrap();
        assert_eq!(day.modrinth, 40);
        assert_eq!(day.curseforge, 75);
    }

    #[test]
    fn per_project_groups_linked_projects_under_one_row() {
        let (conn, _, _) = seed();
        let rows = per_project(&conn, "2026-08-01", "2026-08-11").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].modrinth_downloads, 23_225);
        assert_eq!(rows[0].curseforge_downloads, 86_753);
        assert_eq!(rows[0].title, "Mobs Blocker");
    }

    #[test]
    fn unlinked_projects_appear_alone() {
        let (conn, _, _) = seed();
        upsert(&conn, &ProjectUpsert {
            platform: Platform::Modrinth, ext_id: "solo".into(), slug: Some("solo".into()),
            title: "Solo".into(), project_type: None, url: None, icon_url: None,
            created_at: None, total_downloads: 7, followers: 0,
        }).unwrap();
        let rows = per_project(&conn, "2026-08-01", "2026-08-11").unwrap();
        assert_eq!(rows.len(), 2);
        let solo = rows.iter().find(|r| r.title == "Solo").unwrap();
        assert_eq!(solo.curseforge_downloads, 0);
        assert!(solo.curseforge_id.is_none());
    }

    #[test]
    fn countries_separate_unknown_from_real_codes() {
        let (conn, m, _) = seed();
        upsert_country(&conn, m, "2026-08-10", "DE", 88).unwrap();
        upsert_country(&conn, m, "2026-08-10", "XX", 558).unwrap();
        upsert_country(&conn, m, "2026-08-10", "", 454).unwrap();
        let rows = countries(&conn, "2026-08-01", "2026-08-11").unwrap();
        let unknown = rows.iter().find(|r| r.country == "??").unwrap();
        assert_eq!(unknown.downloads, 1012, "XX et la chaine vide sont fusionnes en ??");
        assert!(rows.iter().any(|r| r.country == "DE" && r.downloads == 88));
    }

    #[test]
    fn loaders_expand_game_version_and_loader_pairs() {
        let (conn, m, _) = seed();
        upsert_version(&conn, m, "v1", Some("1.0"), &["1.20.1".into(), "1.21".into()], &["fabric".into()], 30, None).unwrap();
        upsert_version(&conn, m, "v2", Some("1.1"), &["1.21".into()], &["fabric".into(), "neoforge".into()], 10, None).unwrap();
        let cells = loaders(&conn).unwrap();
        let fabric_121 = cells.iter().find(|c| c.game_version == "1.21" && c.loader == "fabric").unwrap();
        assert_eq!(fabric_121.downloads, 40);
        assert!(cells.iter().any(|c| c.loader == "neoforge" && c.downloads == 10));
    }

    #[test]
    fn kpis_compare_the_two_last_thirty_day_windows() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(100), None, Some("1.5")).unwrap();
        upsert_daily(&conn, m, "2026-06-20", Some(40), None, Some("0.5")).unwrap();
        let k = kpis(&conn, "2026-08-11").unwrap();
        assert_eq!(k.downloads_30d, 100);
        assert_eq!(k.downloads_prev_30d, 40);
        assert_eq!(k.revenue_total, "2.0");
        assert_eq!(k.downloads_modrinth, 23_225);
        assert_eq!(k.downloads_curseforge, 86_753);
    }
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml queries
```

Attendu : `cannot find function timeline`.

- [ ] **Step 3 : écrire l'implémentation au-dessus du bloc de test**

```rust
use crate::error::Result;
use crate::models::{
    CountryTotal, Kpis, LoaderCell, Overview, Platform, ProjectSummary, RevenuePoint, TimelinePoint,
};
use crate::store::metrics::{freshness, recent_events, snapshot_day_count, snapshot_deltas};
use crate::store::projects::{links, list};
use chrono::NaiveDate;
use rusqlite::{params, Connection};
use rust_decimal::Decimal;
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;

/// Ajoute `days` jours à une date `YYYY-MM-DD`. Renvoie la date inchangée si elle est invalide.
pub fn shift_day(day: &str, days: i64) -> String {
    NaiveDate::parse_from_str(day, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.checked_add_signed(chrono::Duration::days(days)))
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| day.to_string())
}

pub fn timeline(conn: &Connection, from: &str, to: &str) -> Result<Vec<TimelinePoint>> {
    let mut per_day: BTreeMap<String, (i64, i64)> = BTreeMap::new();

    let mut stmt = conn.prepare(
        "SELECT day, COALESCE(SUM(downloads), 0) FROM metrics_daily
         WHERE day >= ?1 AND day < ?2 GROUP BY day",
    )?;
    for row in stmt.query_map(params![from, to], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
    })? {
        let (day, downloads) = row?;
        per_day.entry(day).or_default().0 += downloads;
    }

    for ((_, day), delta) in snapshot_deltas(conn)? {
        if day.as_str() >= from && day.as_str() < to {
            per_day.entry(day).or_default().1 += delta;
        }
    }

    Ok(per_day
        .into_iter()
        .map(|(day, (modrinth, curseforge))| TimelinePoint { day, modrinth, curseforge })
        .collect())
}

pub fn per_project(conn: &Connection, from: &str, to: &str) -> Result<Vec<ProjectSummary>> {
    let projects = list(conn)?;
    let links = links(conn)?;
    let by_id: HashMap<i64, &_> = projects.iter().map(|p| (p.id, p)).collect();

    let mut spark_by_project: HashMap<i64, BTreeMap<String, i64>> = HashMap::new();
    let mut stmt = conn.prepare(
        "SELECT project_id, day, COALESCE(downloads, 0) FROM metrics_daily
         WHERE day >= ?1 AND day < ?2",
    )?;
    for row in stmt.query_map(params![from, to], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?))
    })? {
        let (project_id, day, downloads) = row?;
        *spark_by_project.entry(project_id).or_default().entry(day).or_insert(0) += downloads;
    }

    let cf_deltas = snapshot_deltas(conn)?;
    let mut consumed_cf: Vec<i64> = Vec::new();
    let mut out: Vec<ProjectSummary> = Vec::new();

    for project in projects.iter().filter(|p| p.platform == Platform::Modrinth) {
        let link = links.iter().find(|l| l.modrinth_project_id == project.id);
        let cf = link.and_then(|l| by_id.get(&l.cf_project_id).copied());
        if let Some(cf) = cf {
            consumed_cf.push(cf.id);
        }
        let mut spark: BTreeMap<String, i64> =
            spark_by_project.get(&project.id).cloned().unwrap_or_default();
        if let Some(cf) = cf {
            for ((cf_id, day), delta) in &cf_deltas {
                if *cf_id == cf.id && day.as_str() >= from && day.as_str() < to {
                    *spark.entry(day.clone()).or_insert(0) += delta;
                }
            }
        }
        out.push(ProjectSummary {
            key: format!("m{}", project.id),
            title: project.title.clone(),
            icon_url: project.icon_url.clone().or_else(|| cf.and_then(|c| c.icon_url.clone())),
            modrinth_id: Some(project.id),
            curseforge_id: cf.map(|c| c.id),
            modrinth_downloads: project.total_downloads,
            curseforge_downloads: cf.map(|c| c.total_downloads).unwrap_or(0),
            followers: project.followers,
            link_confidence: link.map(|l| l.confidence),
            spark: spark.into_values().collect(),
        });
    }

    for project in projects
        .iter()
        .filter(|p| p.platform == Platform::CurseForge && !consumed_cf.contains(&p.id))
    {
        let spark: BTreeMap<String, i64> = cf_deltas
            .iter()
            .filter(|((cf_id, day), _)| *cf_id == project.id && day.as_str() >= from && day.as_str() < to)
            .map(|((_, day), delta)| (day.clone(), *delta))
            .collect();
        out.push(ProjectSummary {
            key: format!("c{}", project.id),
            title: project.title.clone(),
            icon_url: project.icon_url.clone(),
            modrinth_id: None,
            curseforge_id: Some(project.id),
            modrinth_downloads: 0,
            curseforge_downloads: project.total_downloads,
            followers: 0,
            link_confidence: None,
            spark: spark.into_values().collect(),
        });
    }

    out.sort_by(|a, b| {
        (b.modrinth_downloads + b.curseforge_downloads)
            .cmp(&(a.modrinth_downloads + a.curseforge_downloads))
    });
    Ok(out)
}

pub fn countries(conn: &Connection, from: &str, to: &str) -> Result<Vec<CountryTotal>> {
    let mut stmt = conn.prepare(
        "SELECT CASE WHEN country IN ('', 'XX') THEN '??' ELSE country END AS code,
                SUM(downloads)
         FROM countries_daily WHERE day >= ?1 AND day < ?2
         GROUP BY code ORDER BY SUM(downloads) DESC",
    )?;
    let rows = stmt
        .query_map(params![from, to], |r| {
            Ok(CountryTotal { country: r.get(0)?, downloads: r.get(1)? })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn loaders(conn: &Connection) -> Result<Vec<LoaderCell>> {
    let mut stmt = conn.prepare("SELECT game_versions, loaders, downloads FROM versions")?;
    let mut totals: HashMap<(String, String), i64> = HashMap::new();
    for row in stmt.query_map([], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?))
    })? {
        let (game_versions, loader_list, downloads) = row?;
        let game_versions: Vec<String> = serde_json::from_str(&game_versions).unwrap_or_default();
        let loader_list: Vec<String> = serde_json::from_str(&loader_list).unwrap_or_default();
        for game_version in &game_versions {
            for loader in &loader_list {
                *totals.entry((game_version.clone(), loader.clone())).or_insert(0) += downloads;
            }
        }
    }
    let mut cells: Vec<LoaderCell> = totals
        .into_iter()
        .map(|((game_version, loader), downloads)| LoaderCell { game_version, loader, downloads })
        .collect();
    cells.sort_by(|a, b| b.downloads.cmp(&a.downloads));
    Ok(cells)
}

pub fn revenue(conn: &Connection, from: &str, to: &str) -> Result<Vec<RevenuePoint>> {
    let mut stmt = conn.prepare(
        "SELECT day, revenue FROM metrics_daily
         WHERE day >= ?1 AND day < ?2 AND revenue IS NOT NULL ORDER BY day",
    )?;
    let mut per_day: BTreeMap<String, Decimal> = BTreeMap::new();
    for row in stmt.query_map(params![from, to], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })? {
        let (day, amount) = row?;
        let amount = Decimal::from_str(&amount).unwrap_or_default();
        *per_day.entry(day).or_default() += amount;
    }
    Ok(per_day
        .into_iter()
        .map(|(day, amount)| RevenuePoint { day, amount: amount.to_string() })
        .collect())
}

pub fn kpis(conn: &Connection, today: &str) -> Result<Kpis> {
    let window_start = shift_day(today, -30);
    let previous_start = shift_day(today, -60);

    let sum_downloads = |from: &str, to: &str| -> Result<i64> {
        Ok(conn.query_row(
            "SELECT COALESCE(SUM(downloads), 0) FROM metrics_daily WHERE day >= ?1 AND day < ?2",
            params![from, to],
            |r| r.get(0),
        )?)
    };

    let per_platform = |platform: Platform| -> Result<i64> {
        Ok(conn.query_row(
            "SELECT COALESCE(SUM(total_downloads), 0) FROM projects
             WHERE platform = ?1 AND archived_at IS NULL",
            params![platform.as_str()],
            |r| r.get(0),
        )?)
    };

    let mut stmt = conn.prepare("SELECT revenue FROM metrics_daily WHERE revenue IS NOT NULL")?;
    let mut revenue_total = Decimal::ZERO;
    for row in stmt.query_map([], |r| r.get::<_, String>(0))? {
        revenue_total += Decimal::from_str(&row?).unwrap_or_default();
    }

    let downloads_modrinth = per_platform(Platform::Modrinth)?;
    let downloads_curseforge = per_platform(Platform::CurseForge)?;

    Ok(Kpis {
        downloads_total: downloads_modrinth + downloads_curseforge,
        downloads_modrinth,
        downloads_curseforge,
        downloads_30d: sum_downloads(&window_start, today)?,
        downloads_prev_30d: sum_downloads(&previous_start, &window_start)?,
        revenue_total: revenue_total.normalize().to_string(),
        revenue_pending: "0".into(),
        followers: conn.query_row(
            "SELECT COALESCE(SUM(followers), 0) FROM projects WHERE archived_at IS NULL",
            [],
            |r| r.get(0),
        )?,
        projects_active: conn.query_row(
            "SELECT COUNT(*) FROM projects WHERE archived_at IS NULL",
            [],
            |r| r.get(0),
        )?,
    })
}

pub fn overview(conn: &Connection, today: &str, range_days: i64) -> Result<Overview> {
    let from = shift_day(today, -range_days);
    let to = shift_day(today, 1);
    let mut kpis = kpis(conn, today)?;
    if let Some(balance) = crate::store::metrics::get_meta(conn, "modrinth_balance")? {
        kpis.revenue_pending = balance;
    }

    Ok(Overview {
        kpis,
        timeline: timeline(conn, &from, &to)?,
        per_project: per_project(conn, &from, &to)?,
        countries: countries(conn, &from, &to)?,
        loaders: loaders(conn)?,
        revenue: revenue(conn, &from, &to)?,
        events: recent_events(conn, 40)?,
        freshness: freshness(conn)?,
        curseforge_history_days: snapshot_day_count(conn)?,
    })
}
```

Le solde en attente vient de la clé `modrinth_balance` de la table `meta`, écrite par la synchronisation en Task 12. Tant qu'aucune synchronisation n'a tourné, `revenue_pending` vaut `"0"`.

- [ ] **Step 4 : déclarer le sous-module**

Dans `src-tauri/src/store/mod.rs`, ajoute `pub mod queries;`.

- [ ] **Step 5 : lancer les tests**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml queries
```

Attendu : `test result: ok. 6 passed`.

- [ ] **Step 6 : commit**

```powershell
git add -A
git commit -m "feat: agregations SQL alimentant la page de vision"
```

---

### Task 12 : Orchestration de la synchronisation

**Files:**
- Create: `src-tauri/src/sync.rs`
- Modify: `src-tauri/src/lib.rs`

Trois opérations séparées : découverte, rafraîchissement, snapshot. Chaque provider échoue indépendamment et son résultat est journalisé dans `sync_runs`.

- [ ] **Step 1 : écrire les tests qui échouent**

Dans `src-tauri/src/sync.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::projects::{list, links};
    use crate::store::Store;

    #[test]
    fn analytics_window_starts_after_the_last_known_day() {
        assert_eq!(window_start(Some("2026-08-05"), "2026-08-11", 365), "2026-08-05");
        assert_eq!(window_start(None, "2026-08-11", 30), "2026-07-12");
    }

    #[test]
    fn candidate_username_variants_are_ordered() {
        assert_eq!(
            username_candidates("DreykaOas"),
            vec!["DreykaOas".to_string(), "DreykaOas_official".to_string()]
        );
    }

    #[test]
    fn apply_matches_writes_links_for_identical_titles() {
        let store = Store::open_in_memory().unwrap();
        store.with(|conn| {
            use crate::models::Platform;
            use crate::store::projects::{upsert, ProjectUpsert};
            let mk = |platform, ext: &str, slug: &str, title: &str| ProjectUpsert {
                platform, ext_id: ext.into(), slug: Some(slug.into()), title: title.into(),
                project_type: None, url: None, icon_url: None, created_at: None,
                total_downloads: 0, followers: 0,
            };
            upsert(conn, &mk(Platform::Modrinth, "m1", "mobsblocker", "Mobs Blocker")).unwrap();
            upsert(conn, &mk(Platform::CurseForge, "c1", "mobblocker", "Mobs Blocker")).unwrap();
            apply_matches(conn)
        })
        .unwrap();

        let rows = store.with(links).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].confidence, 1.0);
        assert_eq!(store.with(list).unwrap().len(), 2);
    }
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml sync
```

Attendu : `cannot find function window_start`.

- [ ] **Step 3 : écrire l'implémentation au-dessus du bloc de test**

```rust
use crate::config::{Session, Settings};
use crate::error::{AppError, Result};
use crate::matching::{match_projects, Candidate};
use crate::models::Platform;
use crate::providers::curseforge::{CfFetch, CurseForgeClient};
use crate::providers::modrinth::{timestamp_to_day, ModrinthClient};
use crate::store::metrics as m;
use crate::store::projects as p;
use crate::store::queries::shift_day;
use crate::store::Store;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use rusqlite::Connection;
use serde::Serialize;

/// Fenêtre maximale récupérée lors du tout premier rafraîchissement.
pub const INITIAL_WINDOW_DAYS: i64 = 365;

#[derive(Debug, Clone, Serialize)]
pub struct SyncReport {
    pub provider: String,
    pub status: String,
    pub detail: String,
}

/// Tout ce dont la synchronisation a besoin : la session OAuth et les réglages.
pub struct SyncContext {
    pub session: Session,
    pub settings: Settings,
}

pub fn today_utc() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

pub fn window_start(last_known_day: Option<&str>, today: &str, fallback_days: i64) -> String {
    match last_known_day {
        Some(day) => day.to_string(),
        None => shift_day(today, -fallback_days),
    }
}

pub fn username_candidates(modrinth_username: &str) -> Vec<String> {
    vec![
        modrinth_username.to_string(),
        format!("{modrinth_username}_official"),
    ]
}

fn to_datetime(day: &str) -> DateTime<Utc> {
    NaiveDate::parse_from_str(day, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .and_then(|d| Utc.from_local_datetime(&d).single())
        .unwrap_or_else(Utc::now)
}

pub fn apply_matches(conn: &Connection) -> Result<usize> {
    let projects = p::list(conn)?;
    let to_candidate = |platform: Platform| -> Vec<Candidate> {
        projects
            .iter()
            .filter(|x| x.platform == platform && x.archived_at.is_none())
            .map(|x| Candidate { id: x.id, slug: x.slug.clone(), title: x.title.clone() })
            .collect()
    };

    p::clear_automatic_links(conn)?;
    let matches = match_projects(&to_candidate(Platform::Modrinth), &to_candidate(Platform::CurseForge));
    for found in &matches {
        p::upsert_link(conn, found.modrinth_id, found.curseforge_id, found.confidence, false)?;
    }
    Ok(matches.len())
}

async fn run_provider<F>(store: &Store, provider: &str, work: F) -> SyncReport
where
    F: std::future::Future<Output = Result<String>>,
{
    let started = Utc::now().to_rfc3339();
    let run_id = store
        .with(|conn| m::start_sync_run(conn, provider, &started))
        .unwrap_or(0);

    let (status, detail) = match work.await {
        Ok(detail) => ("ok".to_string(), detail),
        Err(e) => ("failed".to_string(), e.to_string()),
    };

    let finished = Utc::now().to_rfc3339();
    let _ = store.with(|conn| m::finish_sync_run(conn, run_id, &finished, &status, &detail));
    SyncReport { provider: provider.to_string(), status, detail }
}

async fn discover_modrinth(store: &Store, ctx: &SyncContext) -> Result<String> {
    let client = ModrinthClient::new(&ctx.session.token)?;
    let user = client.me().await?;
    let projects = client.projects(&user.id).await?;
    let now = Utc::now().to_rfc3339();
    let seen: Vec<String> = projects.iter().map(|x| x.id.clone()).collect();

    store.with(|conn| {
        for project in &projects {
            p::upsert(conn, &p::ProjectUpsert {
                platform: Platform::Modrinth,
                ext_id: project.id.clone(),
                slug: Some(project.slug.clone()),
                title: project.title.clone(),
                project_type: project.project_type.clone(),
                url: Some(format!("https://modrinth.com/mod/{}", project.slug)),
                icon_url: project.icon_url.clone(),
                created_at: project.published.clone(),
                total_downloads: project.downloads,
                followers: project.followers,
            })?;
        }
        p::archive_missing(conn, Platform::Modrinth, &seen, &now)?;
        m::set_meta(conn, "modrinth_balance", &user.payout_data.balance.to_string())?;
        Ok(())
    })?;

    Ok(format!("{} projets", projects.len()))
}

async fn discover_curseforge(store: &Store, ctx: &SyncContext) -> Result<String> {
    let client = CurseForgeClient::new()?;
    let candidates = match ctx.settings.curseforge_username.as_deref() {
        Some(name) => vec![name.to_string()],
        None => username_candidates(&ctx.session.username),
    };

    let mut author = None;
    for candidate in &candidates {
        if candidate.is_empty() {
            continue;
        }
        if let Ok(found) = client.author(candidate).await {
            author = Some(found);
            break;
        }
    }
    let author = author.ok_or_else(|| {
        AppError::Config(format!("aucun auteur CurseForge trouvé parmi : {}", candidates.join(", ")))
    })?;

    let now = Utc::now().to_rfc3339();
    let mut seen: Vec<String> = Vec::new();
    let mut queued = 0usize;

    for entry in &author.projects {
        match client.project(entry.id).await? {
            CfFetch::Queued => queued += 1,
            CfFetch::Ready(project) => {
                seen.push(project.id.to_string());
                store.with(|conn| {
                    p::upsert(conn, &p::ProjectUpsert {
                        platform: Platform::CurseForge,
                        ext_id: project.id.to_string(),
                        slug: project.slug.clone(),
                        title: project.title.clone(),
                        project_type: project.project_type.clone(),
                        url: project.url.clone(),
                        icon_url: project.thumbnail.clone(),
                        created_at: project.created_at.clone(),
                        total_downloads: project.downloads_total,
                        followers: 0,
                    })?;
                    m::insert_snapshot(
                        conn,
                        conn.query_row(
                            "SELECT id FROM projects WHERE platform = 'curseforge' AND ext_id = ?1",
                            rusqlite::params![project.id.to_string()],
                            |r| r.get::<_, i64>(0),
                        )?,
                        &now,
                        project.downloads_total,
                        Some(project.downloads_monthly),
                    )
                })?;
            }
        }
    }

    if !seen.is_empty() {
        store.with(|conn| p::archive_missing(conn, Platform::CurseForge, &seen, &now))?;
    }
    store.with(|conn| m::set_meta(conn, "curseforge_username", &author.username))?;

    Ok(format!("{} projets, {queued} en file d'attente", seen.len()))
}

async fn refresh_modrinth(store: &Store, ctx: &SyncContext) -> Result<String> {
    let client = ModrinthClient::new(&ctx.session.token)?;
    let user_id = ctx.session.user_id.clone();

    let rows = store.with(|conn| p::list_by_platform(conn, Platform::Modrinth))?;
    let ids: Vec<String> = rows.iter().map(|r| r.ext_id.clone()).collect();
    if ids.is_empty() {
        return Ok("aucun projet".into());
    }
    let by_ext: std::collections::HashMap<String, i64> =
        rows.iter().map(|r| (r.ext_id.clone(), r.id)).collect();

    let today = today_utc();
    let last = store.with(m::last_metrics_day)?;
    let start = to_datetime(&window_start(last.as_deref(), &today, INITIAL_WINDOW_DAYS));
    let end = to_datetime(&shift_day(&today, 1));

    let downloads = client.analytics_downloads(&ids, start, end).await?;
    let views = client.analytics_views(&ids, start, end).await?;
    let revenue = client.analytics_revenue(&ids, start, end).await?;
    let countries = client.analytics_countries(&ids, start, end).await?;

    store.with(|conn| {
        for (ext_id, series) in &downloads {
            let Some(project_id) = by_ext.get(ext_id) else { continue };
            for (ts, value) in series {
                m::upsert_daily(conn, *project_id, &timestamp_to_day(*ts), Some(*value), None, None)?;
            }
        }
        for (ext_id, series) in &views {
            let Some(project_id) = by_ext.get(ext_id) else { continue };
            for (ts, value) in series {
                m::upsert_daily(conn, *project_id, &timestamp_to_day(*ts), None, Some(*value), None)?;
            }
        }
        for (ext_id, series) in &revenue {
            let Some(project_id) = by_ext.get(ext_id) else { continue };
            for (ts, value) in series {
                m::upsert_daily(
                    conn, *project_id, &timestamp_to_day(*ts), None, None, Some(&value.to_string()),
                )?;
            }
        }
        for (ext_id, per_country) in &countries {
            let Some(project_id) = by_ext.get(ext_id) else { continue };
            for (code, value) in per_country {
                m::upsert_country(conn, *project_id, &today, code, *value)?;
            }
        }
        Ok(())
    })?;

    for row in &rows {
        let versions = client.versions(&row.ext_id).await?;
        store.with(|conn| {
            for version in &versions {
                m::upsert_version(
                    conn, row.id, &version.id, version.version_number.as_deref(),
                    &version.game_versions, &version.loaders, version.downloads,
                    version.date_published.as_deref(),
                )?;
            }
            Ok(())
        })?;
    }

    let notifications = client.notifications(&user_id).await?;
    store.with(|conn| {
        for notification in &notifications {
            let project_id = notification.project_ext_id.as_ref().and_then(|ext| by_ext.get(ext).copied());
            let title = project_id
                .and_then(|id| rows.iter().find(|r| r.id == id))
                .map(|r| r.title.clone())
                .unwrap_or_else(|| "Modrinth".into());
            m::insert_event(
                conn, "modrinth", &notification.occurred_at, &notification.kind,
                project_id, &title, &notification.detail,
            )?;
        }
        Ok(())
    })?;

    Ok(format!("{} projets, {} notifications", rows.len(), notifications.len()))
}

async fn snapshot_curseforge(store: &Store) -> Result<String> {
    let client = CurseForgeClient::new()?;
    let rows = store.with(|conn| p::list_by_platform(conn, Platform::CurseForge))?;
    let now = Utc::now().to_rfc3339();
    let mut written = 0usize;
    let mut queued = 0usize;

    for row in &rows {
        let Ok(id) = row.ext_id.parse::<i64>() else { continue };
        match client.project(id).await? {
            CfFetch::Queued => queued += 1,
            CfFetch::Ready(project) => {
                store.with(|conn| {
                    m::insert_snapshot(
                        conn, row.id, &now, project.downloads_total, Some(project.downloads_monthly),
                    )?;
                    p::upsert(conn, &p::ProjectUpsert {
                        platform: Platform::CurseForge,
                        ext_id: row.ext_id.clone(),
                        slug: project.slug.clone(),
                        title: project.title.clone(),
                        project_type: project.project_type.clone(),
                        url: project.url.clone(),
                        icon_url: project.thumbnail.clone(),
                        created_at: project.created_at.clone(),
                        total_downloads: project.downloads_total,
                        followers: 0,
                    })?;
                    Ok(())
                })?;
                written += 1;
            }
        }
    }
    Ok(format!("{written} snapshots, {queued} en file d'attente"))
}

/// Cycle complet : découverte, appariement, rafraîchissement, snapshot.
/// Aucun échec ne bloque les autres providers.
pub async fn full(store: &Store, ctx: &SyncContext) -> Vec<SyncReport> {
    let mut reports = Vec::new();
    reports.push(run_provider(store, "modrinth", discover_modrinth(store, ctx)).await);
    reports.push(run_provider(store, "curseforge", discover_curseforge(store, ctx)).await);

    if let Err(e) = store.with(apply_matches) {
        reports.push(SyncReport {
            provider: "matching".into(),
            status: "failed".into(),
            detail: e.to_string(),
        });
    }

    reports.push(run_provider(store, "modrinth-analytics", refresh_modrinth(store, ctx)).await);
    reports.push(run_provider(store, "curseforge-snapshot", snapshot_curseforge(store)).await);
    reports
}
```

- [ ] **Step 4 : déclarer le module**

Dans `src-tauri/src/lib.rs`, ajoute `pub mod sync;`.

- [ ] **Step 5 : lancer les tests**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml sync
```

Attendu : `test result: ok. 3 passed`.

- [ ] **Step 6 : commit**

```powershell
git add -A
git commit -m "feat: orchestration de la decouverte, du rafraichissement et des snapshots"
```

---

### Task 13 : Surface Tauri

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1 : écrire `src-tauri/src/commands.rs`**

```rust
use crate::config::{self, OauthApp, Settings};
use crate::error::{AppError, Result};
use crate::models::Overview;
use crate::oauth;
use crate::store::{projects as p, queries, Store};
use crate::sync::{self, SyncContext, SyncReport};
use serde::Serialize;
use std::path::PathBuf;
use tauri::{Manager, State};
use tauri_plugin_opener::OpenerExt;

/// Injectés à la compilation. Absents, l'écran de réglages prend le relais.
const COMPILED_CLIENT_ID: Option<&str> = option_env!("MODRINTH_CLIENT_ID");
const COMPILED_CLIENT_SECRET: Option<&str> = option_env!("MODRINTH_CLIENT_SECRET");

pub struct AppState {
    pub store: Store,
    pub data_dir: PathBuf,
}

impl AppState {
    pub fn oauth_app(&self) -> Option<OauthApp> {
        config::load_oauth_app(&self.data_dir, COMPILED_CLIENT_ID, COMPILED_CLIENT_SECRET)
    }

    pub fn context(&self) -> Result<SyncContext> {
        Ok(SyncContext {
            session: config::require_token(&self.data_dir)?,
            settings: config::load_settings(&self.data_dir),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct AuthStatus {
    pub connected: bool,
    pub username: Option<String>,
    pub connected_since: Option<String>,
    /// Faux quand le binaire a été compilé sans identifiants et qu'aucun fichier ne les fournit.
    pub oauth_app_configured: bool,
}

#[tauri::command]
pub fn auth_status(state: State<'_, AppState>) -> AuthStatus {
    let session = config::load_session(&state.data_dir);
    AuthStatus {
        connected: session.is_some(),
        username: session.as_ref().map(|s| s.username.clone()),
        connected_since: session.as_ref().map(|s| s.obtained_at.clone()),
        oauth_app_configured: state.oauth_app().is_some(),
    }
}

#[tauri::command]
pub async fn login(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<AuthStatus> {
    let oauth_app = state.oauth_app().ok_or_else(|| {
        AppError::Config(
            "aucune application OAuth configurée : enregistre-en une sur modrinth.com/settings/applications".into(),
        )
    })?;

    let opener = app.clone();
    let session = oauth::login(&oauth_app, move |url| {
        opener
            .opener()
            .open_url(url, None::<&str>)
            .map_err(|e| AppError::Config(format!("ouverture du navigateur : {e}")))
    })
    .await?;

    config::save_session(&state.data_dir, &session)?;
    Ok(auth_status(state))
}

#[tauri::command]
pub fn logout(state: State<'_, AppState>) -> Result<AuthStatus> {
    config::clear_session(&state.data_dir)?;
    Ok(auth_status(state))
}

#[tauri::command]
pub fn save_oauth_app(state: State<'_, AppState>, client_id: String, client_secret: String) -> Result<()> {
    let app = OauthApp {
        client_id: client_id.trim().to_string(),
        client_secret: client_secret.trim().to_string(),
    };
    if app.client_id.is_empty() || app.client_secret.is_empty() {
        return Err(AppError::Config("client_id et client_secret sont requis".into()));
    }
    config::save_oauth_app(&state.data_dir, &app)
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    config::load_settings(&state.data_dir)
}

#[tauri::command]
pub fn save_settings(
    state: State<'_, AppState>,
    curseforge_username: Option<String>,
    range_days: i64,
) -> Result<()> {
    let settings = Settings {
        curseforge_username: curseforge_username
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty()),
        range_days: range_days.clamp(7, 730),
    };
    config::save_settings(&state.data_dir, &settings)
}

#[tauri::command]
pub async fn sync_now(state: State<'_, AppState>) -> Result<Vec<SyncReport>> {
    let ctx = state.context()?;
    Ok(sync::full(&state.store, &ctx).await)
}

#[tauri::command]
pub fn overview(state: State<'_, AppState>, range_days: i64) -> Result<Overview> {
    let today = sync::today_utc();
    let range = range_days.clamp(7, 730);
    state.store.with(|conn| queries::overview(conn, &today, range))
}

#[tauri::command]
pub fn link_manual(state: State<'_, AppState>, modrinth_id: i64, curseforge_id: i64) -> Result<()> {
    state
        .store
        .with(|conn| p::upsert_link(conn, modrinth_id, curseforge_id, 1.0, true))
}

#[tauri::command]
pub fn unlink(state: State<'_, AppState>, modrinth_id: i64, curseforge_id: i64) -> Result<()> {
    state
        .store
        .with(|conn| p::delete_link(conn, modrinth_id, curseforge_id).map(|_| ()))
}

#[tauri::command]
pub fn unlinked_projects(state: State<'_, AppState>) -> Result<Vec<(i64, String, String)>> {
    state.store.with(|conn| {
        let links = p::links(conn)?;
        Ok(p::list(conn)?
            .into_iter()
            .filter(|project| match project.platform {
                crate::models::Platform::Modrinth => {
                    !links.iter().any(|l| l.modrinth_project_id == project.id)
                }
                crate::models::Platform::CurseForge => {
                    !links.iter().any(|l| l.cf_project_id == project.id)
                }
            })
            .map(|project| (project.id, project.platform.as_str().to_string(), project.title))
            .collect())
    })
}

pub fn data_dir(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
}
```

- [ ] **Step 2 : monter l'état et les commandes dans `src-tauri/src/lib.rs`**

Remplace intégralement le contenu de `src-tauri/src/lib.rs` par :

```rust
pub mod commands;
pub mod config;
pub mod error;
pub mod matching;
pub mod models;
pub mod oauth;
pub mod providers;
pub mod store;
pub mod sync;

use commands::AppState;
use store::Store;
use tauri::Manager;

pub fn run() {
    #[cfg(debug_assertions)]
    tracing_subscriber::fmt().with_env_filter("chartographer_lib=debug").init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = commands::data_dir(app.handle());
            std::fs::create_dir_all(&data_dir).ok();
            let store = Store::open(&config::db_path(&data_dir))?;
            app.manage(AppState { store, data_dir });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth_status,
            commands::login,
            commands::logout,
            commands::save_oauth_app,
            commands::get_settings,
            commands::save_settings,
            commands::sync_now,
            commands::overview,
            commands::link_manual,
            commands::unlink,
            commands::unlinked_projects,
        ])
        .run(tauri::generate_context!())
        .expect("erreur au démarrage de Tauri");
}
```

- [ ] **Step 3 : vérifier la compilation et la suite complète**

```powershell
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Attendu : clippy sort sans avertissement, tous les tests passent.

- [ ] **Step 4 : commit**

```powershell
git add -A
git commit -m "feat: commandes Tauri et etat applicatif"
```

---

### Task 14 : Couche front — types, API, état, formatage

**Files:**
- Create: `src/lib/types.ts`, `src/lib/api.ts`, `src/lib/format.ts`, `src/lib/state.svelte.ts`
- Create: `src/lib/format.test.ts`
- Create: `vitest.config.ts`

- [ ] **Step 1 : écrire le test qui échoue**

`src/lib/format.test.ts` :

```ts
import { describe, expect, it } from "vitest";
import { compactNumber, deltaPercent, formatDay, formatMoney } from "./format";

describe("compactNumber", () => {
  it("abrège au-delà du millier", () => {
    expect(compactNumber(999)).toBe("999");
    expect(compactNumber(1776)).toBe("1,8 k");
    expect(compactNumber(176968)).toBe("177,0 k");
    expect(compactNumber(2_400_000)).toBe("2,4 M");
  });
});

describe("deltaPercent", () => {
  it("renvoie null quand la période précédente est vide", () => {
    expect(deltaPercent(100, 0)).toBeNull();
  });
  it("calcule la variation relative", () => {
    expect(deltaPercent(150, 100)).toBe(50);
    expect(deltaPercent(50, 100)).toBe(-50);
  });
});

describe("formatMoney", () => {
  it("arrondit à deux décimales sans perdre les petits montants", () => {
    expect(formatMoney("0.00762273691987854525")).toBe("0,01 $");
    expect(formatMoney("12.5")).toBe("12,50 $");
    expect(formatMoney("nope")).toBe("0,00 $");
  });
});

describe("formatDay", () => {
  it("rend un jour ISO en jour court", () => {
    expect(formatDay("2026-08-11")).toBe("11 août");
  });
});
```

- [ ] **Step 2 : créer `vitest.config.ts` puis lancer le test**

```ts
import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte({ hot: false })],
  test: { environment: "node", include: ["src/**/*.test.ts"] },
});
```

```powershell
npm test
```

Attendu : échec, `Cannot find module './format'`.

- [ ] **Step 3 : écrire `src/lib/format.ts`**

```ts
const NBSP = " ";

export function compactNumber(value: number): string {
  const abs = Math.abs(value);
  if (abs >= 1_000_000) return `${(value / 1_000_000).toFixed(1).replace(".", ",")}${NBSP}M`;
  if (abs >= 1_000) return `${(value / 1_000).toFixed(1).replace(".", ",")}${NBSP}k`;
  return String(value);
}

export function deltaPercent(current: number, previous: number): number | null {
  if (previous === 0) return null;
  return Math.round(((current - previous) / previous) * 100);
}

export function formatMoney(raw: string): string {
  const value = Number.parseFloat(raw);
  const safe = Number.isFinite(value) ? value : 0;
  return `${safe.toFixed(2).replace(".", ",")}${NBSP}$`;
}

const MONTHS = [
  "janv.", "févr.", "mars", "avril", "mai", "juin",
  "juil.", "août", "sept.", "oct.", "nov.", "déc.",
];

export function formatDay(iso: string): string {
  const [, month, day] = iso.split("-");
  const index = Number.parseInt(month ?? "1", 10) - 1;
  return `${Number.parseInt(day ?? "1", 10)} ${MONTHS[index] ?? ""}`;
}

export function countryLabel(code: string): string {
  if (code === "??") return "Inconnu";
  try {
    return new Intl.DisplayNames(["fr"], { type: "region" }).of(code) ?? code;
  } catch {
    return code;
  }
}
```

Note : le test attend `1,8 k` avec une espace fine insécable ` ` entre le nombre et l'unité. Écris le test avec ce caractère exact, ou remplace les assertions par `expect(compactNumber(1776).replace(/\s/g, " ")).toBe("1,8 k")`. Choisis la seconde forme, plus lisible, et applique-la aux quatre assertions de `compactNumber` ainsi qu'aux trois de `formatMoney`.

- [ ] **Step 4 : relancer le test**

```powershell
npm test
```

Attendu : `Test Files 1 passed`, 4 tests verts.

- [ ] **Step 5 : écrire `src/lib/types.ts`**

```ts
export interface Kpis {
  downloads_total: number;
  downloads_modrinth: number;
  downloads_curseforge: number;
  downloads_30d: number;
  downloads_prev_30d: number;
  revenue_total: string;
  revenue_pending: string;
  followers: number;
  projects_active: number;
}

export interface TimelinePoint { day: string; modrinth: number; curseforge: number }

export interface ProjectSummary {
  key: string;
  title: string;
  icon_url: string | null;
  modrinth_id: number | null;
  curseforge_id: number | null;
  modrinth_downloads: number;
  curseforge_downloads: number;
  followers: number;
  link_confidence: number | null;
  spark: number[];
}

export interface CountryTotal { country: string; downloads: number }
export interface LoaderCell { game_version: string; loader: string; downloads: number }
export interface RevenuePoint { day: string; amount: string }
export interface EventRow { occurred_at: string; kind: string; title: string; detail: string }
export interface Freshness { provider: string; status: string; finished_at: string | null; detail: string }

export interface Overview {
  kpis: Kpis;
  timeline: TimelinePoint[];
  per_project: ProjectSummary[];
  countries: CountryTotal[];
  loaders: LoaderCell[];
  revenue: RevenuePoint[];
  events: EventRow[];
  freshness: Freshness[];
  curseforge_history_days: number;
}

export interface AuthStatus {
  connected: boolean;
  username: string | null;
  connected_since: string | null;
  oauth_app_configured: boolean;
}

export interface Settings {
  curseforge_username: string | null;
  range_days: number;
}

export interface SyncReport { provider: string; status: string; detail: string }
export interface AppErrorPayload { kind: string; message: string }
```

- [ ] **Step 6 : écrire `src/lib/api.ts`**

```ts
import { invoke } from "@tauri-apps/api/core";
import type { AuthStatus, Overview, Settings, SyncReport } from "./types";

export const api = {
  authStatus: () => invoke<AuthStatus>("auth_status"),
  login: () => invoke<AuthStatus>("login"),
  logout: () => invoke<AuthStatus>("logout"),
  saveOauthApp: (clientId: string, clientSecret: string) =>
    invoke<void>("save_oauth_app", { clientId, clientSecret }),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (curseforgeUsername: string | null, rangeDays: number) =>
    invoke<void>("save_settings", { curseforgeUsername, rangeDays }),
  syncNow: () => invoke<SyncReport[]>("sync_now"),
  overview: (rangeDays: number) => invoke<Overview>("overview", { rangeDays }),
  linkManual: (modrinthId: number, curseforgeId: number) =>
    invoke<void>("link_manual", { modrinthId, curseforgeId }),
  unlink: (modrinthId: number, curseforgeId: number) =>
    invoke<void>("unlink", { modrinthId, curseforgeId }),
  unlinkedProjects: () => invoke<[number, string, string][]>("unlinked_projects"),
};
```

- [ ] **Step 7 : écrire `src/lib/state.svelte.ts`**

```ts
import { api } from "./api";
import type { AppErrorPayload, AuthStatus, Overview, SyncReport } from "./types";

class Dashboard {
  auth = $state<AuthStatus | null>(null);
  overview = $state<Overview | null>(null);
  rangeDays = $state(90);
  loading = $state(false);
  syncing = $state(false);
  connecting = $state(false);
  error = $state<string | null>(null);
  lastSync = $state<SyncReport[]>([]);
  selectedProject = $state<string | null>(null);

  async refreshAuth() {
    this.auth = await api.authStatus();
  }

  /// Ouvre le navigateur et attend le retour. La commande ne rend la main
  /// qu'une fois la redirection recue ou le delai depasse.
  async login() {
    this.connecting = true;
    this.error = null;
    try {
      this.auth = await api.login();
      await this.sync();
    } catch (e) {
      this.error = (e as AppErrorPayload)?.message ?? String(e);
    } finally {
      this.connecting = false;
    }
  }

  async logout() {
    this.auth = await api.logout();
    this.overview = null;
  }

  async load() {
    this.loading = true;
    this.error = null;
    try {
      this.overview = await api.overview(this.rangeDays);
    } catch (e) {
      this.error = (e as AppErrorPayload)?.message ?? String(e);
    } finally {
      this.loading = false;
    }
  }

  async setRange(days: number) {
    this.rangeDays = days;
    await this.load();
  }

  async sync() {
    this.syncing = true;
    this.error = null;
    try {
      this.lastSync = await api.syncNow();
      await this.load();
    } catch (e) {
      this.error = (e as AppErrorPayload)?.message ?? String(e);
    } finally {
      this.syncing = false;
    }
  }
}

export const dashboard = new Dashboard();
```

- [ ] **Step 8 : vérifier et commit**

```powershell
npm run check
npm test
git add -A
git commit -m "feat: couche front typee, etat en runes et formatage teste"
```

---

### Task 15 : Wrapper ECharts et constructeurs d'options

**Files:**
- Create: `src/lib/charts/Chart.svelte`, `src/lib/charts/timeline.ts`, `src/lib/charts/worldmap.ts`, `src/lib/charts/split.ts`, `src/lib/charts/heatmap.ts`, `src/lib/charts/revenue.ts`, `src/lib/charts/sparkline.ts`, `src/lib/charts/theme.ts`
- Create: `src/lib/charts/options.test.ts`

Les constructeurs d'options sont des fonctions pures, testées sans DOM. Le wrapper est le seul point qui touche ECharts.

- [ ] **Step 1 : écrire les tests qui échouent**

`src/lib/charts/options.test.ts` :

```ts
import { describe, expect, it } from "vitest";
import { timelineOption } from "./timeline";
import { splitOption } from "./split";
import { heatmapOption } from "./heatmap";
import { worldMapOption } from "./worldmap";

const points = [
  { day: "2026-08-09", modrinth: 40, curseforge: 0 },
  { day: "2026-08-10", modrinth: 55, curseforge: 75 },
];

describe("timelineOption", () => {
  it("produit deux séries empilées alignées sur les jours", () => {
    const option = timelineOption(points, true);
    expect(option.xAxis.data).toEqual(["2026-08-09", "2026-08-10"]);
    expect(option.series).toHaveLength(2);
    expect(option.series[0].data).toEqual([40, 55]);
    expect(option.series[1].data).toEqual([0, 75]);
    expect(option.series[0].stack).toBe(option.series[1].stack);
  });

  it("désempile quand le mode comparaison est actif", () => {
    const option = timelineOption(points, false);
    expect(option.series[0].stack).toBeUndefined();
  });
});

describe("splitOption", () => {
  it("trie par écart de plateforme décroissant", () => {
    const option = splitOption([
      { key: "a", title: "Petit", modrinth_downloads: 10, curseforge_downloads: 12 },
      { key: "b", title: "Gros", modrinth_downloads: 23225, curseforge_downloads: 86753 },
    ] as never);
    expect(option.yAxis.data[0]).toBe("Gros");
  });
});

describe("heatmapOption", () => {
  it("indexe les cellules sur les axes des versions et des loaders", () => {
    const option = heatmapOption([
      { game_version: "1.21", loader: "fabric", downloads: 40 },
      { game_version: "1.20.1", loader: "neoforge", downloads: 10 },
    ]);
    expect(option.xAxis.data).toContain("1.21");
    expect(option.yAxis.data).toContain("neoforge");
    expect(option.series[0].data).toHaveLength(2);
    expect(option.visualMap.max).toBe(40);
  });
});

describe("worldMapOption", () => {
  it("exclut le pays inconnu de la carte", () => {
    const option = worldMapOption([
      { country: "DE", downloads: 88 },
      { country: "??", downloads: 1012 },
    ]);
    expect(option.series[0].data.map((d: { name: string }) => d.name)).toEqual(["DE"]);
  });
});
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

```powershell
npm test
```

Attendu : `Cannot find module './timeline'`.

- [ ] **Step 3 : écrire `src/lib/charts/theme.ts`**

```ts
export const COLORS = {
  modrinth: "#00af5c",
  curseforge: "#f16436",
  accent: "#5ac8a8",
  text: "#e6ebf0",
  textDim: "#8b97a5",
  grid: "#262d36",
  surface: "#14181d",
};

export const BASE_GRID = { left: 48, right: 16, top: 24, bottom: 56, containLabel: true };

export const AXIS_STYLE = {
  axisLine: { lineStyle: { color: COLORS.grid } },
  axisLabel: { color: COLORS.textDim },
  splitLine: { lineStyle: { color: COLORS.grid, opacity: 0.4 } },
};

export const TOOLTIP = {
  backgroundColor: COLORS.surface,
  borderColor: COLORS.grid,
  textStyle: { color: COLORS.text },
};
```

- [ ] **Step 4 : écrire les constructeurs d'options**

`src/lib/charts/timeline.ts` :

```ts
import type { TimelinePoint } from "../types";
import { AXIS_STYLE, BASE_GRID, COLORS, TOOLTIP } from "./theme";

export function timelineOption(points: TimelinePoint[], stacked: boolean) {
  const stack = stacked ? "downloads" : undefined;
  return {
    grid: BASE_GRID,
    tooltip: { trigger: "axis", ...TOOLTIP },
    legend: { data: ["Modrinth", "CurseForge"], textStyle: { color: COLORS.textDim }, top: 0 },
    xAxis: { type: "category", data: points.map((p) => p.day), ...AXIS_STYLE },
    yAxis: { type: "value", ...AXIS_STYLE },
    dataZoom: [
      { type: "inside", start: 0, end: 100 },
      { type: "slider", height: 20, bottom: 8, borderColor: COLORS.grid, textStyle: { color: COLORS.textDim } },
    ],
    series: [
      {
        name: "Modrinth",
        type: "line",
        stack,
        smooth: true,
        showSymbol: false,
        areaStyle: { opacity: 0.25 },
        itemStyle: { color: COLORS.modrinth },
        data: points.map((p) => p.modrinth),
      },
      {
        name: "CurseForge",
        type: "line",
        stack,
        smooth: true,
        showSymbol: false,
        areaStyle: { opacity: 0.25 },
        itemStyle: { color: COLORS.curseforge },
        data: points.map((p) => p.curseforge),
      },
    ],
  };
}
```

`src/lib/charts/split.ts` :

```ts
import type { ProjectSummary } from "../types";
import { AXIS_STYLE, BASE_GRID, COLORS, TOOLTIP } from "./theme";

export function splitOption(projects: ProjectSummary[]) {
  const rows = [...projects]
    .sort(
      (a, b) =>
        b.modrinth_downloads + b.curseforge_downloads -
        (a.modrinth_downloads + a.curseforge_downloads),
    )
    .slice(0, 15)
    .reverse();

  return {
    grid: { ...BASE_GRID, left: 140 },
    tooltip: { trigger: "axis", axisPointer: { type: "shadow" }, ...TOOLTIP },
    legend: { data: ["Modrinth", "CurseForge"], textStyle: { color: COLORS.textDim }, top: 0 },
    xAxis: { type: "value", ...AXIS_STYLE },
    yAxis: { type: "category", data: rows.map((r) => r.title), ...AXIS_STYLE },
    series: [
      {
        name: "Modrinth",
        type: "bar",
        stack: "total",
        itemStyle: { color: COLORS.modrinth },
        data: rows.map((r) => r.modrinth_downloads),
      },
      {
        name: "CurseForge",
        type: "bar",
        stack: "total",
        itemStyle: { color: COLORS.curseforge },
        data: rows.map((r) => r.curseforge_downloads),
      },
    ],
  };
}
```

`src/lib/charts/heatmap.ts` :

```ts
import type { LoaderCell } from "../types";
import { AXIS_STYLE, COLORS, TOOLTIP } from "./theme";

/** Trie les versions de jeu par ordre numérique décroissant plutôt qu'alphabétique. */
function sortGameVersions(values: string[]): string[] {
  return [...values].sort((a, b) => {
    const pa = a.split(".").map((n) => Number.parseInt(n, 10) || 0);
    const pb = b.split(".").map((n) => Number.parseInt(n, 10) || 0);
    for (let i = 0; i < Math.max(pa.length, pb.length); i += 1) {
      const diff = (pa[i] ?? 0) - (pb[i] ?? 0);
      if (diff !== 0) return diff;
    }
    return 0;
  });
}

export function heatmapOption(cells: LoaderCell[]) {
  const gameVersions = sortGameVersions([...new Set(cells.map((c) => c.game_version))]);
  const loaders = [...new Set(cells.map((c) => c.loader))].sort();
  const max = cells.reduce((acc, c) => Math.max(acc, c.downloads), 0);

  return {
    grid: { left: 90, right: 20, top: 16, bottom: 70, containLabel: true },
    tooltip: { position: "top", ...TOOLTIP },
    xAxis: { type: "category", data: gameVersions, axisLabel: { color: COLORS.textDim, rotate: 45 }, ...AXIS_STYLE },
    yAxis: { type: "category", data: loaders, ...AXIS_STYLE },
    visualMap: {
      min: 0,
      max,
      calculable: false,
      orient: "horizontal",
      left: "center",
      bottom: 0,
      textStyle: { color: COLORS.textDim },
      inRange: { color: ["#14181d", COLORS.accent] },
    },
    series: [
      {
        type: "heatmap",
        data: cells.map((c) => [
          gameVersions.indexOf(c.game_version),
          loaders.indexOf(c.loader),
          c.downloads,
        ]),
        emphasis: { itemStyle: { borderColor: COLORS.text, borderWidth: 1 } },
      },
    ],
  };
}
```

`src/lib/charts/worldmap.ts` :

```ts
import type { CountryTotal } from "../types";
import { COLORS, TOOLTIP } from "./theme";

/** Le code `??` agrège `XX` et la chaîne vide côté Rust : il n'a pas de géométrie. */
export function worldMapOption(countries: CountryTotal[]) {
  const mapped = countries.filter((c) => c.country !== "??");
  const max = mapped.reduce((acc, c) => Math.max(acc, c.downloads), 0);

  return {
    tooltip: { trigger: "item", ...TOOLTIP },
    visualMap: {
      min: 0,
      max: Math.max(max, 1),
      left: 12,
      bottom: 12,
      calculable: true,
      textStyle: { color: COLORS.textDim },
      inRange: { color: ["#1b2027", COLORS.accent, COLORS.curseforge] },
    },
    series: [
      {
        type: "map",
        map: "world",
        roam: true,
        itemStyle: { areaColor: "#161b21", borderColor: COLORS.grid },
        emphasis: { label: { show: false }, itemStyle: { areaColor: COLORS.accent } },
        nameProperty: "iso_a2",
        data: mapped.map((c) => ({ name: c.country, value: c.downloads })),
      },
    ],
  };
}
```

`src/lib/charts/revenue.ts` :

```ts
import type { RevenuePoint } from "../types";
import { AXIS_STYLE, BASE_GRID, COLORS, TOOLTIP } from "./theme";

export function revenueOption(points: RevenuePoint[]) {
  const daily = points.map((p) => Number.parseFloat(p.amount) || 0);
  let running = 0;
  const cumulative = daily.map((v) => {
    running += v;
    return Number(running.toFixed(4));
  });

  return {
    grid: BASE_GRID,
    tooltip: { trigger: "axis", ...TOOLTIP },
    legend: { data: ["Journalier", "Cumulé"], textStyle: { color: COLORS.textDim }, top: 0 },
    xAxis: { type: "category", data: points.map((p) => p.day), ...AXIS_STYLE },
    yAxis: [
      { type: "value", ...AXIS_STYLE },
      { type: "value", ...AXIS_STYLE },
    ],
    series: [
      { name: "Journalier", type: "bar", itemStyle: { color: COLORS.accent }, data: daily },
      {
        name: "Cumulé",
        type: "line",
        yAxisIndex: 1,
        smooth: true,
        showSymbol: false,
        itemStyle: { color: COLORS.curseforge },
        data: cumulative,
      },
    ],
  };
}
```

`src/lib/charts/sparkline.ts` :

```ts
import { COLORS } from "./theme";

export function sparklineOption(values: number[]) {
  return {
    grid: { left: 0, right: 0, top: 2, bottom: 2 },
    xAxis: { type: "category", show: false, data: values.map((_, i) => i) },
    yAxis: { type: "value", show: false },
    series: [
      {
        type: "line",
        smooth: true,
        showSymbol: false,
        lineStyle: { width: 1.5, color: COLORS.accent },
        areaStyle: { opacity: 0.18, color: COLORS.accent },
        data: values,
      },
    ],
  };
}
```

- [ ] **Step 5 : écrire `src/lib/charts/Chart.svelte`**

```svelte
<script lang="ts">
  import * as echarts from "echarts";
  import { onDestroy } from "svelte";

  let { option, height = 320 }: { option: unknown; height?: number } = $props();

  let container = $state<HTMLDivElement | null>(null);
  let chart: echarts.ECharts | null = null;
  let observer: ResizeObserver | null = null;

  $effect(() => {
    if (!container) return;
    if (!chart) {
      chart = echarts.init(container, null, { renderer: "canvas" });
      observer = new ResizeObserver(() => chart?.resize());
      observer.observe(container);
    }
    chart.setOption(option as echarts.EChartsOption, { notMerge: true });
  });

  onDestroy(() => {
    observer?.disconnect();
    chart?.dispose();
    chart = null;
  });
</script>

<div bind:this={container} style="height: {height}px; width: 100%;"></div>
```

- [ ] **Step 6 : relancer les tests**

```powershell
npm test
```

Attendu : `Test Files 2 passed`, tous les tests verts.

- [ ] **Step 7 : commit**

```powershell
git add -A
git commit -m "feat: wrapper ECharts et constructeurs d'options testes"
```

---

### Task 16 : Composants de la page de vision

**Files:**
- Create: `src/lib/components/KpiBand.svelte`, `Timeline.svelte`, `WorldMap.svelte`, `PlatformSplit.svelte`, `LoaderHeatmap.svelte`, `RevenueChart.svelte`, `ProjectsTable.svelte`, `EventsFeed.svelte`, `FreshnessBadge.svelte`, `Card.svelte`
- Create: `src/lib/views/Vision.svelte`

La carte mondiale a besoin d'une géométrie. `world-atlas` fournit un TopoJSON de 110 m, converti en GeoJSON par `topojson-client` puis enregistré une fois auprès d'ECharts. Le pays `??` n'a volontairement pas de géométrie et s'affiche hors carte.

- [ ] **Step 1 : `src/lib/components/Card.svelte`**

```svelte
<script lang="ts">
  let { title, subtitle = "", children }: {
    title: string;
    subtitle?: string;
    children: import("svelte").Snippet;
  } = $props();
</script>

<section>
  <header>
    <h2>{title}</h2>
    {#if subtitle}<p>{subtitle}</p>{/if}
  </header>
  {@render children()}
</section>

<style>
  section {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 16px;
  }
  header { margin-bottom: 12px; }
  h2 { margin: 0; font-size: 0.95rem; font-weight: 600; letter-spacing: 0.01em; }
  p { margin: 4px 0 0; font-size: 0.8rem; color: var(--text-dim); }
</style>
```

- [ ] **Step 2 : `src/lib/components/KpiBand.svelte`**

```svelte
<script lang="ts">
  import { compactNumber, deltaPercent, formatMoney } from "../format";
  import type { Kpis } from "../types";

  let { kpis }: { kpis: Kpis } = $props();

  const delta = $derived(deltaPercent(kpis.downloads_30d, kpis.downloads_prev_30d));
  const tiles = $derived([
    { label: "Téléchargements", value: compactNumber(kpis.downloads_total), hint: `${compactNumber(kpis.downloads_modrinth)} Modrinth · ${compactNumber(kpis.downloads_curseforge)} CurseForge` },
    { label: "30 derniers jours", value: compactNumber(kpis.downloads_30d), hint: delta === null ? "pas de période de référence" : `${delta > 0 ? "+" : ""}${delta} % vs 30 j précédents` },
    { label: "Revenus cumulés", value: formatMoney(kpis.revenue_total), hint: `${formatMoney(kpis.revenue_pending)} en attente` },
    { label: "Followers", value: compactNumber(kpis.followers), hint: `${kpis.projects_active} projets actifs` },
  ]);
</script>

<div class="band">
  {#each tiles as tile (tile.label)}
    <article>
      <span class="label">{tile.label}</span>
      <strong>{tile.value}</strong>
      <span class="hint">{tile.hint}</span>
    </article>
  {/each}
</div>

<style>
  .band { display: grid; grid-template-columns: repeat(auto-fit, minmax(210px, 1fr)); gap: 12px; }
  article {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .label { font-size: 0.75rem; color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.06em; }
  strong { font-size: 1.7rem; font-weight: 600; font-variant-numeric: tabular-nums; }
  .hint { font-size: 0.78rem; color: var(--text-dim); }
</style>
```

- [ ] **Step 3 : les composants de graphique**

`src/lib/components/Timeline.svelte` :

```svelte
<script lang="ts">
  import Chart from "../charts/Chart.svelte";
  import { timelineOption } from "../charts/timeline";
  import type { TimelinePoint } from "../types";

  let { points }: { points: TimelinePoint[] } = $props();
  let stacked = $state(true);
  const option = $derived(timelineOption(points, stacked));
</script>

<label>
  <input type="checkbox" bind:checked={stacked} />
  Empiler les plateformes
</label>
<Chart {option} height={340} />

<style>
  label { display: inline-flex; align-items: center; gap: 6px; font-size: 0.8rem; color: var(--text-dim); margin-bottom: 8px; }
</style>
```

`src/lib/components/PlatformSplit.svelte` :

```svelte
<script lang="ts">
  import Chart from "../charts/Chart.svelte";
  import { splitOption } from "../charts/split";
  import type { ProjectSummary } from "../types";

  let { projects }: { projects: ProjectSummary[] } = $props();
  const option = $derived(splitOption(projects));
</script>

<Chart {option} height={Math.max(240, Math.min(projects.length, 15) * 26 + 90)} />
```

`src/lib/components/LoaderHeatmap.svelte` :

```svelte
<script lang="ts">
  import Chart from "../charts/Chart.svelte";
  import { heatmapOption } from "../charts/heatmap";
  import type { LoaderCell } from "../types";

  let { cells }: { cells: LoaderCell[] } = $props();
  const option = $derived(heatmapOption(cells));
</script>

{#if cells.length === 0}
  <p class="empty">Aucune version indexée. Lance une synchronisation.</p>
{:else}
  <Chart {option} height={300} />
{/if}

<style>
  .empty { color: var(--text-dim); font-size: 0.85rem; margin: 0; }
</style>
```

`src/lib/components/RevenueChart.svelte` :

```svelte
<script lang="ts">
  import Chart from "../charts/Chart.svelte";
  import { revenueOption } from "../charts/revenue";
  import type { RevenuePoint } from "../types";

  let { points }: { points: RevenuePoint[] } = $props();
  const option = $derived(revenueOption(points));
</script>

{#if points.length === 0}
  <p class="empty">Aucun revenu sur la période.</p>
{:else}
  <Chart {option} height={280} />
{/if}

<style>
  .empty { color: var(--text-dim); font-size: 0.85rem; margin: 0; }
</style>
```

- [ ] **Step 4 : `src/lib/components/WorldMap.svelte`**

```svelte
<script lang="ts">
  import * as echarts from "echarts";
  import { feature } from "topojson-client";
  import worldTopology from "world-atlas/countries-110m.json";
  import Chart from "../charts/Chart.svelte";
  import { worldMapOption } from "../charts/worldmap";
  import { compactNumber, countryLabel } from "../format";
  import type { CountryTotal } from "../types";

  let { countries }: { countries: CountryTotal[] } = $props();

  // La geometrie de world-atlas porte des identifiants numeriques ISO-3166-1,
  // alors que Modrinth renvoie des codes ISO-2. On enregistre la carte une fois
  // en projetant les identifiants numeriques vers leur code alpha-2.
  let registered = false;
  function ensureMap() {
    if (registered) return;
    const collection = feature(
      worldTopology as never,
      (worldTopology as never as { objects: { countries: never } }).objects.countries,
    ) as unknown as GeoJSON.FeatureCollection;
    for (const item of collection.features) {
      const numeric = String(item.id ?? "").padStart(3, "0");
      item.properties = { ...item.properties, iso_a2: NUMERIC_TO_ALPHA2[numeric] ?? numeric };
    }
    echarts.registerMap("world", collection as never);
    registered = true;
  }

  ensureMap();

  const unknown = $derived(countries.find((c) => c.country === "??"));
  const option = $derived(worldMapOption(countries));
  const top = $derived(countries.filter((c) => c.country !== "??").slice(0, 6));
</script>

<Chart {option} height={380} />

<div class="side">
  <ul>
    {#each top as row (row.country)}
      <li><span>{countryLabel(row.country)}</span><b>{compactNumber(row.downloads)}</b></li>
    {/each}
  </ul>
  {#if unknown}
    <p class="unknown">Origine inconnue : {compactNumber(unknown.downloads)} téléchargements, non représentés sur la carte.</p>
  {/if}
</div>

<style>
  .side { margin-top: 10px; display: flex; flex-direction: column; gap: 8px; }
  ul { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: repeat(auto-fit, minmax(140px, 1fr)); gap: 4px 14px; }
  li { display: flex; justify-content: space-between; font-size: 0.82rem; color: var(--text-dim); }
  li b { color: var(--text); font-variant-numeric: tabular-nums; }
  .unknown { margin: 0; font-size: 0.78rem; color: var(--warn); }
</style>
```

Crée à côté `src/lib/components/iso.ts` avec la table `NUMERIC_TO_ALPHA2` et importe-la dans le composant. Génère-la une fois avec ce script, qui écrit le fichier depuis les données de `world-atlas` et la table ISO de Node :

```powershell
node --input-type=module -e "
import { readFileSync, writeFileSync } from 'node:fs';
const topo = JSON.parse(readFileSync('node_modules/world-atlas/countries-110m.json','utf8'));
const names = topo.objects.countries.geometries.map(g => [String(g.id).padStart(3,'0'), g.properties.name]);
const regions = new Intl.DisplayNames(['en'], { type: 'region' });
const alpha2 = [...Array(26*26)].map((_,i)=>String.fromCharCode(65+Math.floor(i/26))+String.fromCharCode(65+i%26));
const byName = new Map(alpha2.map(c => { try { return [regions.of(c), c]; } catch { return [null, c]; } }));
const table = Object.fromEntries(names.map(([id,name]) => [id, byName.get(name) ?? null]).filter(([,c]) => c));
writeFileSync('src/lib/components/iso.ts', 'export const NUMERIC_TO_ALPHA2: Record<string, string> = ' + JSON.stringify(table, null, 2) + ';\n');
console.log(Object.keys(table).length + ' pays mappes');
"
```

Attendu : au moins 150 pays mappés. Ajoute ensuite `import { NUMERIC_TO_ALPHA2 } from "./iso";` en tête de `WorldMap.svelte`.

- [ ] **Step 5 : `src/lib/components/ProjectsTable.svelte`**

```svelte
<script lang="ts">
  import Chart from "../charts/Chart.svelte";
  import { sparklineOption } from "../charts/sparkline";
  import { compactNumber } from "../format";
  import type { ProjectSummary } from "../types";

  let { projects, onselect }: {
    projects: ProjectSummary[];
    onselect: (key: string) => void;
  } = $props();

  type Column = "title" | "total" | "modrinth" | "curseforge" | "followers";
  let sortBy = $state<Column>("total");
  let ascending = $state(false);

  const total = (p: ProjectSummary) => p.modrinth_downloads + p.curseforge_downloads;

  const rows = $derived(
    [...projects].sort((a, b) => {
      const direction = ascending ? 1 : -1;
      switch (sortBy) {
        case "title": return a.title.localeCompare(b.title) * direction;
        case "modrinth": return (a.modrinth_downloads - b.modrinth_downloads) * direction;
        case "curseforge": return (a.curseforge_downloads - b.curseforge_downloads) * direction;
        case "followers": return (a.followers - b.followers) * direction;
        default: return (total(a) - total(b)) * direction;
      }
    }),
  );

  function sort(column: Column) {
    if (sortBy === column) ascending = !ascending;
    else { sortBy = column; ascending = false; }
  }
</script>

<table>
  <thead>
    <tr>
      <th><button onclick={() => sort("title")}>Projet</button></th>
      <th>Tendance</th>
      <th><button onclick={() => sort("modrinth")}>Modrinth</button></th>
      <th><button onclick={() => sort("curseforge")}>CurseForge</button></th>
      <th><button onclick={() => sort("total")}>Total</button></th>
      <th><button onclick={() => sort("followers")}>Followers</button></th>
    </tr>
  </thead>
  <tbody>
    {#each rows as row (row.key)}
      <tr onclick={() => onselect(row.key)}>
        <td class="name">
          {#if row.icon_url}<img src={row.icon_url} alt="" />{/if}
          <span>{row.title}</span>
          {#if row.link_confidence !== null && row.link_confidence < 1}
            <em title="Appariement automatique incertain">lien ~{Math.round(row.link_confidence * 100)} %</em>
          {/if}
          {#if row.curseforge_id === null}<em class="solo">Modrinth seul</em>{/if}
          {#if row.modrinth_id === null}<em class="solo">CurseForge seul</em>{/if}
        </td>
        <td class="spark">
          {#if row.spark.length > 1}<Chart option={sparklineOption(row.spark)} height={30} />{/if}
        </td>
        <td>{compactNumber(row.modrinth_downloads)}</td>
        <td>{compactNumber(row.curseforge_downloads)}</td>
        <td><b>{compactNumber(row.modrinth_downloads + row.curseforge_downloads)}</b></td>
        <td>{row.followers}</td>
      </tr>
    {/each}
  </tbody>
</table>

<style>
  table { width: 100%; border-collapse: collapse; font-size: 0.85rem; }
  th { text-align: right; padding: 6px 10px; border-bottom: 1px solid var(--border); }
  th:first-child, td:first-child { text-align: left; }
  th button { background: none; border: 0; color: var(--text-dim); font: inherit; cursor: pointer; padding: 0; }
  th button:hover { color: var(--text); }
  td { padding: 6px 10px; border-bottom: 1px solid var(--border); text-align: right; font-variant-numeric: tabular-nums; }
  tbody tr { cursor: pointer; }
  tbody tr:hover { background: var(--surface-2); }
  .name { display: flex; align-items: center; gap: 8px; }
  .name img { width: 22px; height: 22px; border-radius: 5px; }
  .spark { width: 110px; }
  em { font-style: normal; font-size: 0.7rem; color: var(--warn); border: 1px solid var(--warn); border-radius: 4px; padding: 1px 5px; }
  em.solo { color: var(--text-dim); border-color: var(--border); }
</style>
```

- [ ] **Step 6 : `EventsFeed.svelte` et `FreshnessBadge.svelte`**

```svelte
<script lang="ts">
  import type { EventRow } from "../types";
  let { events }: { events: EventRow[] } = $props();
</script>

{#if events.length === 0}
  <p class="empty">Aucun évènement.</p>
{:else}
  <ul>
    {#each events as event (event.occurred_at + event.title)}
      <li>
        <time>{event.occurred_at.slice(0, 10)}</time>
        <span class="kind">{event.kind}</span>
        <b>{event.title}</b>
        <span class="detail">{event.detail}</span>
      </li>
    {/each}
  </ul>
{/if}

<style>
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 8px; max-height: 380px; overflow-y: auto; }
  li { display: grid; grid-template-columns: 84px 110px 1fr; gap: 8px; align-items: baseline; font-size: 0.82rem; }
  time, .kind, .detail { color: var(--text-dim); }
  .kind { font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.05em; }
  .detail { grid-column: 1 / -1; }
  .empty { color: var(--text-dim); font-size: 0.85rem; margin: 0; }
</style>
```

`FreshnessBadge.svelte` :

```svelte
<script lang="ts">
  import type { Freshness } from "../types";
  let { entries }: { entries: Freshness[] } = $props();
</script>

<div>
  {#each entries as entry (entry.provider)}
    <span class:ok={entry.status === "ok"} class:ko={entry.status !== "ok"} title={entry.detail}>
      {entry.provider} · {entry.finished_at ? entry.finished_at.slice(0, 16).replace("T", " ") : "jamais"}
    </span>
  {/each}
</div>

<style>
  div { display: flex; flex-wrap: wrap; gap: 6px; }
  span { font-size: 0.72rem; border-radius: 999px; padding: 2px 9px; border: 1px solid var(--border); color: var(--text-dim); }
  .ok { border-color: var(--modrinth); color: var(--modrinth); }
  .ko { border-color: var(--error); color: var(--error); }
</style>
```

- [ ] **Step 7 : `src/lib/views/Vision.svelte`**

```svelte
<script lang="ts">
  import Card from "../components/Card.svelte";
  import EventsFeed from "../components/EventsFeed.svelte";
  import FreshnessBadge from "../components/FreshnessBadge.svelte";
  import KpiBand from "../components/KpiBand.svelte";
  import LoaderHeatmap from "../components/LoaderHeatmap.svelte";
  import PlatformSplit from "../components/PlatformSplit.svelte";
  import ProjectsTable from "../components/ProjectsTable.svelte";
  import RevenueChart from "../components/RevenueChart.svelte";
  import Timeline from "../components/Timeline.svelte";
  import WorldMap from "../components/WorldMap.svelte";
  import { dashboard } from "../state.svelte";

  const RANGES = [30, 90, 180, 365];
  const overview = $derived(dashboard.overview);
</script>

{#if overview}
  <div class="toolbar">
    <div class="ranges">
      {#each RANGES as days (days)}
        <button class:active={dashboard.rangeDays === days} onclick={() => dashboard.setRange(days)}>
          {days} j
        </button>
      {/each}
    </div>
    <FreshnessBadge entries={overview.freshness} />
    <button class="sync" onclick={() => dashboard.sync()} disabled={dashboard.syncing}>
      {dashboard.syncing ? "Synchronisation…" : "Synchroniser"}
    </button>
  </div>

  <KpiBand kpis={overview.kpis} />

  {#if overview.curseforge_history_days < 2}
    <p class="notice">
      L'historique CurseForge se construit par snapshots quotidiens :
      {overview.curseforge_history_days} jour(s) enregistré(s). La courbe CurseForge restera plate
      jusqu'au deuxième snapshot.
    </p>
  {/if}

  <div class="grid">
    <Card title="Téléchargements par jour" subtitle="Modrinth en série, CurseForge reconstruit par snapshots">
      <Timeline points={overview.timeline} />
    </Card>

    <Card title="Origine des téléchargements">
      <WorldMap countries={overview.countries} />
    </Card>

    <Card title="Modrinth contre CurseForge" subtitle="Total par projet, trié par volume">
      <PlatformSplit projects={overview.per_project} />
    </Card>

    <Card title="Versions de jeu et loaders" subtitle="Concentration des téléchargements Modrinth">
      <LoaderHeatmap cells={overview.loaders} />
    </Card>

    <Card title="Revenus" subtitle="Journalier et cumulé">
      <RevenueChart points={overview.revenue} />
    </Card>

    <Card title="Évènements">
      <EventsFeed events={overview.events} />
    </Card>

    <div class="wide">
      <Card title="Tous les projets" subtitle="Clique une ligne pour le détail">
        <ProjectsTable
          projects={overview.per_project}
          onselect={(key) => (dashboard.selectedProject = key)}
        />
      </Card>
    </div>
  </div>
{:else if dashboard.loading}
  <p class="notice">Chargement…</p>
{:else}
  <p class="notice">Aucune donnée. Lance une synchronisation.</p>
{/if}

<style>
  .toolbar { display: flex; align-items: center; gap: 14px; flex-wrap: wrap; margin-bottom: 14px; }
  .ranges { display: flex; gap: 4px; }
  button {
    background: var(--surface); border: 1px solid var(--border); color: var(--text-dim);
    border-radius: 7px; padding: 5px 12px; font: inherit; font-size: 0.8rem; cursor: pointer;
  }
  button.active, button:hover { color: var(--text); border-color: var(--accent); }
  .sync { margin-left: auto; color: var(--text); }
  .sync:disabled { opacity: 0.5; cursor: default; }
  .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(440px, 1fr)); gap: 14px; margin-top: 14px; }
  .wide { grid-column: 1 / -1; }
  .notice {
    margin: 14px 0 0; padding: 10px 14px; border-radius: var(--radius);
    background: var(--surface-2); border: 1px solid var(--border);
    color: var(--text-dim); font-size: 0.83rem;
  }
</style>
```

- [ ] **Step 8 : vérifier et commit**

```powershell
npm run check
npm test
git add -A
git commit -m "feat: page de vision et ses composants"
```

---

### Task 17 : Connexion, réglages, détail de projet, routage

**Files:**
- Create: `src/lib/views/Login.svelte`, `src/lib/views/Settings.svelte`, `src/lib/views/ProjectDetail.svelte`
- Modify: `src/App.svelte`

- [ ] **Step 1 : `src/lib/views/Login.svelte`**

Écran de premier lancement. Un seul bouton. Le formulaire d'identifiants d'application n'apparaît que si le binaire a été compilé sans.

```svelte
<script lang="ts">
  import { api } from "../api";
  import { dashboard } from "../state.svelte";

  let clientId = $state("");
  let clientSecret = $state("");
  let saving = $state(false);

  const needsApp = $derived(dashboard.auth?.oauth_app_configured === false);

  async function saveApp() {
    saving = true;
    try {
      await api.saveOauthApp(clientId, clientSecret);
      await dashboard.refreshAuth();
    } catch (e) {
      dashboard.error = (e as { message?: string })?.message ?? String(e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="screen">
  <h1>Chartographer</h1>
  <p class="tagline">Tes statistiques Modrinth et CurseForge sur un seul écran.</p>

  {#if needsApp}
    <div class="setup">
      <p>
        Enregistre une application OAuth sur
        <code>modrinth.com/settings/applications</code>
        avec <code>http://127.0.0.1/callback</code> comme URL de redirection, puis colle ses
        identifiants ici. Cette étape disparaît si tu compiles l'application avec
        <code>MODRINTH_CLIENT_ID</code> et <code>MODRINTH_CLIENT_SECRET</code>.
      </p>
      <input bind:value={clientId} placeholder="client_id" />
      <input bind:value={clientSecret} type="password" placeholder="client_secret" />
      <button onclick={saveApp} disabled={saving || !clientId || !clientSecret}>Enregistrer</button>
    </div>
  {:else}
    <button class="primary" onclick={() => dashboard.login()} disabled={dashboard.connecting}>
      {dashboard.connecting ? "En attente du navigateur…" : "Se connecter avec Modrinth"}
    </button>
    <p class="hint">
      Ton navigateur va s'ouvrir sur la page d'autorisation Modrinth. Rien à copier, rien à coller.
    </p>
  {/if}

  {#if dashboard.error}<p class="error">{dashboard.error}</p>{/if}
</div>

<style>
  .screen { min-height: 100vh; display: grid; place-content: center; justify-items: center; gap: 10px; padding: 24px; text-align: center; }
  h1 { margin: 0; font-size: 2rem; font-weight: 600; }
  .tagline { margin: 0 0 18px; color: var(--text-dim); }
  .primary { background: var(--accent); color: #08110d; border: 0; border-radius: 9px; padding: 12px 26px; font: inherit; font-weight: 600; cursor: pointer; }
  .primary:disabled { opacity: 0.6; cursor: default; }
  .hint { color: var(--text-dim); font-size: 0.82rem; max-width: 36ch; }
  .setup { display: flex; flex-direction: column; gap: 8px; max-width: 48ch; }
  .setup p { color: var(--text-dim); font-size: 0.84rem; line-height: 1.5; text-align: left; }
  code { background: var(--surface-2); border-radius: 4px; padding: 1px 5px; font-size: 0.9em; }
  input { background: var(--surface); border: 1px solid var(--border); border-radius: 7px; color: var(--text); padding: 9px 11px; font: inherit; }
  button { background: var(--surface); border: 1px solid var(--border); color: var(--text); border-radius: 7px; padding: 9px 14px; font: inherit; cursor: pointer; }
  .error { color: var(--error); font-size: 0.84rem; }
</style>
```

- [ ] **Step 2 : `src/lib/views/Settings.svelte`**

```svelte
<script lang="ts">
  import { api } from "../api";
  import { dashboard } from "../state.svelte";
  import type { Settings } from "../types";

  let settings = $state<Settings>({ curseforge_username: null, range_days: 90 });
  let unlinked = $state<[number, string, string][]>([]);
  let message = $state("");

  $effect(() => {
    api.getSettings().then((value) => (settings = value));
    api.unlinkedProjects().then((value) => (unlinked = value));
  });

  async function save() {
    await api.saveSettings(settings.curseforge_username, settings.range_days);
    message = "Réglages enregistrés.";
  }

  async function link(modrinthId: number, curseforgeId: number) {
    await api.linkManual(modrinthId, curseforgeId);
    unlinked = await api.unlinkedProjects();
    await dashboard.load();
  }

  const modrinthOrphans = $derived(unlinked.filter(([, platform]) => platform === "modrinth"));
  const curseforgeOrphans = $derived(unlinked.filter(([, platform]) => platform === "curseforge"));

  let leftId = $state<number | null>(null);
  let rightId = $state<number | null>(null);
</script>

<h1>Réglages</h1>

<section>
  <h2>Compte Modrinth</h2>
  {#if dashboard.auth?.connected}
    <p>Connecté en tant que <b>{dashboard.auth.username}</b> depuis {dashboard.auth.connected_since?.slice(0, 10)}.</p>
    <button onclick={() => dashboard.logout()}>Se déconnecter</button>
  {:else}
    <button onclick={() => dashboard.login()}>Se connecter avec Modrinth</button>
  {/if}
</section>

<section>
  <h2>CurseForge</h2>
  <p class="hint">Détecté automatiquement depuis ton pseudo Modrinth. Renseigne-le seulement si la détection échoue.</p>
  <input
    value={settings.curseforge_username ?? ""}
    oninput={(e) => (settings.curseforge_username = e.currentTarget.value || null)}
    placeholder="pseudo auteur CurseForge"
  />
</section>

<section>
  <h2>Fenêtre d'historique</h2>
  <input type="number" min="7" max="730" bind:value={settings.range_days} />
  <span class="hint">jours affichés par défaut</span>
</section>

<section>
  <h2>Appariements manquants</h2>
  {#if modrinthOrphans.length === 0 && curseforgeOrphans.length === 0}
    <p class="hint">Tous les projets sont appariés.</p>
  {:else}
    <div class="pair">
      <select bind:value={leftId}>
        <option value={null}>Projet Modrinth…</option>
        {#each modrinthOrphans as [id, , title] (id)}<option value={id}>{title}</option>{/each}
      </select>
      <select bind:value={rightId}>
        <option value={null}>Projet CurseForge…</option>
        {#each curseforgeOrphans as [id, , title] (id)}<option value={id}>{title}</option>{/each}
      </select>
      <button disabled={leftId === null || rightId === null} onclick={() => link(leftId!, rightId!)}>
        Apparier
      </button>
    </div>
  {/if}
</section>

<button class="primary" onclick={save}>Enregistrer</button>
{#if message}<p class="ok">{message}</p>{/if}

<style>
  h1 { font-size: 1.4rem; margin: 0 0 18px; }
  h2 { font-size: 0.95rem; margin: 0 0 8px; }
  section { background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 16px; margin-bottom: 12px; max-width: 720px; }
  p { margin: 0 0 10px; font-size: 0.86rem; }
  .hint { color: var(--text-dim); font-size: 0.8rem; }
  input, select { background: var(--surface-2); border: 1px solid var(--border); border-radius: 7px; color: var(--text); padding: 8px 10px; font: inherit; font-size: 0.86rem; }
  button { background: var(--surface-2); border: 1px solid var(--border); color: var(--text); border-radius: 7px; padding: 8px 14px; font: inherit; font-size: 0.86rem; cursor: pointer; }
  .primary { background: var(--accent); color: #08110d; border: 0; font-weight: 600; }
  .pair { display: flex; gap: 8px; flex-wrap: wrap; }
  .ok { color: var(--modrinth); font-size: 0.84rem; }
</style>
```

- [ ] **Step 3 : `src/lib/views/ProjectDetail.svelte`**

```svelte
<script lang="ts">
  import Card from "../components/Card.svelte";
  import Chart from "../charts/Chart.svelte";
  import { sparklineOption } from "../charts/sparkline";
  import { compactNumber } from "../format";
  import { dashboard } from "../state.svelte";

  const project = $derived(
    dashboard.overview?.per_project.find((p) => p.key === dashboard.selectedProject) ?? null,
  );
  const total = $derived(
    project ? project.modrinth_downloads + project.curseforge_downloads : 0,
  );
  const share = $derived(
    total === 0 ? 0 : Math.round((project!.modrinth_downloads / total) * 100),
  );
</script>

{#if project}
  <button class="back" onclick={() => (dashboard.selectedProject = null)}>← Retour</button>
  <h1>{project.title}</h1>

  <div class="grid">
    <Card title="Répartition par plateforme">
      <p class="big">{share} % Modrinth · {100 - share} % CurseForge</p>
      <p class="hint">
        {compactNumber(project.modrinth_downloads)} contre
        {compactNumber(project.curseforge_downloads)} téléchargements
      </p>
    </Card>

    <Card title="Tendance sur la période">
      {#if project.spark.length > 1}
        <Chart option={sparklineOption(project.spark)} height={140} />
      {:else}
        <p class="hint">Pas encore assez de points.</p>
      {/if}
    </Card>

    <Card title="Appariement">
      {#if project.link_confidence === null}
        <p class="hint">Projet mono-plateforme. Apparie-le depuis les réglages si son jumeau existe.</p>
      {:else}
        <p class="big">{Math.round(project.link_confidence * 100)} %</p>
        <p class="hint">confiance de l'appariement automatique</p>
      {/if}
    </Card>
  </div>
{/if}

<style>
  .back { background: none; border: 0; color: var(--text-dim); font: inherit; cursor: pointer; padding: 0; margin-bottom: 8px; }
  h1 { font-size: 1.4rem; margin: 0 0 16px; }
  .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 14px; }
  .big { font-size: 1.3rem; margin: 0; font-variant-numeric: tabular-nums; }
  .hint { color: var(--text-dim); font-size: 0.82rem; margin: 6px 0 0; }
</style>
```

- [ ] **Step 4 : réécrire `src/App.svelte`**

```svelte
<script lang="ts">
  import Login from "./lib/views/Login.svelte";
  import ProjectDetail from "./lib/views/ProjectDetail.svelte";
  import Settings from "./lib/views/Settings.svelte";
  import Vision from "./lib/views/Vision.svelte";
  import { dashboard } from "./lib/state.svelte";

  let view = $state<"vision" | "settings">("vision");
  let ready = $state(false);

  $effect(() => {
    if (ready) return;
    ready = true;
    dashboard.refreshAuth().then(() => {
      if (dashboard.auth?.connected) dashboard.load();
    });
  });
</script>

{#if !dashboard.auth}
  <p class="boot">Démarrage…</p>
{:else if !dashboard.auth.connected}
  <Login />
{:else}
  <nav>
    <strong>Chartographer</strong>
    <button class:active={view === "vision"} onclick={() => { view = "vision"; dashboard.selectedProject = null; }}>Vision</button>
    <button class:active={view === "settings"} onclick={() => (view = "settings")}>Réglages</button>
    <span class="user">{dashboard.auth.username}</span>
  </nav>

  <main>
    {#if dashboard.error}<p class="error">{dashboard.error}</p>{/if}
    {#if view === "settings"}
      <Settings />
    {:else if dashboard.selectedProject}
      <ProjectDetail />
    {:else}
      <Vision />
    {/if}
  </main>
{/if}

<style>
  nav {
    display: flex; align-items: center; gap: 10px;
    padding: 10px 20px; border-bottom: 1px solid var(--border); background: var(--surface);
  }
  nav strong { margin-right: 12px; }
  nav button { background: none; border: 0; color: var(--text-dim); font: inherit; font-size: 0.86rem; cursor: pointer; padding: 4px 8px; border-radius: 6px; }
  nav button.active, nav button:hover { color: var(--text); background: var(--surface-2); }
  .user { margin-left: auto; color: var(--text-dim); font-size: 0.82rem; }
  main { padding: 18px 20px 40px; }
  .boot { padding: 24px; color: var(--text-dim); }
  .error { background: var(--surface-2); border: 1px solid var(--error); color: var(--error); border-radius: var(--radius); padding: 10px 14px; font-size: 0.84rem; }
</style>
```

- [ ] **Step 5 : vérifier et commit**

```powershell
npm run check
npm test
git add -A
git commit -m "feat: ecran de connexion, reglages, detail de projet et routage"
```

---

### Task 18 : Intégration continue et publication

**Files:**
- Create: `.github/workflows/ci.yml`, `.github/workflows/release.yml`

- [ ] **Step 1 : `.github/workflows/ci.yml`**

```yaml
name: ci

on:
  push:
    branches: [main]
  pull_request:

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: actions/setup-node@v7
        with:
          node-version: 24
          cache: npm
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri
      - name: Dependances systeme Tauri
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
      - run: npm ci
      - run: npm run check
      - run: npm test
      - run: cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
      - run: cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
      - run: cargo test --manifest-path src-tauri/Cargo.toml
```

- [ ] **Step 2 : `.github/workflows/release.yml`**

```yaml
name: release

on:
  push:
    tags: ["v*"]
  workflow_dispatch:

permissions:
  contents: write

jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: ubuntu-latest
            args: "--bundles deb,rpm"
          - platform: windows-latest
            args: "--bundles nsis"
    runs-on: ${{ matrix.platform }}
    steps:
      - uses: actions/checkout@v7
      - uses: actions/setup-node@v7
        with:
          node-version: 24
          cache: npm
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri
      - name: Dependances systeme Tauri
        if: matrix.platform == 'ubuntu-latest'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf rpm
      - run: npm ci
      - uses: tauri-apps/tauri-action@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          MODRINTH_CLIENT_ID: ${{ secrets.MODRINTH_CLIENT_ID }}
          MODRINTH_CLIENT_SECRET: ${{ secrets.MODRINTH_CLIENT_SECRET }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: "Chartographer ${{ github.ref_name }}"
          releaseBody: "Installeurs .deb, .rpm et .exe."
          releaseDraft: true
          prerelease: false
          args: ${{ matrix.args }}
```

Les deux secrets `MODRINTH_CLIENT_ID` et `MODRINTH_CLIENT_SECRET` sont à créer dans les réglages du dépôt. Sans eux, les binaires publiés demandent les identifiants d'application au premier lancement au lieu de proposer directement la connexion.

- [ ] **Step 3 : vérifier la syntaxe des workflows et commit**

```powershell
gh workflow list
git add -A
git commit -m "ci: controle sur push et publication des installeurs sur tag"
git push
```

- [ ] **Step 4 : publier une première version**

```powershell
git tag v0.1.0
git push origin v0.1.0
gh run watch
```

Attendu : deux jobs verts, une Release brouillon contenant `.deb`, `.rpm` et `.exe`.
