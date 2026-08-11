export type ThemeMode = "auto" | "light" | "dark";

const STORAGE_KEY = "chartographer:theme";
const MODES: ThemeMode[] = ["auto", "light", "dark"];

function readMode(): ThemeMode {
  if (typeof localStorage === "undefined") return "auto";
  const stored = localStorage.getItem(STORAGE_KEY);
  return MODES.includes(stored as ThemeMode) ? (stored as ThemeMode) : "auto";
}

function systemPrefersDark(): boolean {
  if (typeof window === "undefined" || !window.matchMedia) return true;
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

class Theme {
  mode = $state<ThemeMode>(readMode());
  systemDark = $state(systemPrefersDark());

  /** Thème réellement appliqué une fois le mode automatique résolu. */
  dark = $derived(this.mode === "auto" ? this.systemDark : this.mode === "dark");

  constructor() {
    if (typeof window !== "undefined" && window.matchMedia) {
      window
        .matchMedia("(prefers-color-scheme: dark)")
        .addEventListener("change", (e) => (this.systemDark = e.matches));
    }
    this.applyAttribute();
  }

  /**
   * En mode automatique on retire l'attribut : la règle `prefers-color-scheme`
   * reprend la main. Sinon l'attribut force le thème dans les deux sens.
   */
  applyAttribute() {
    if (typeof document === "undefined") return;
    if (this.mode === "auto") {
      delete document.documentElement.dataset.theme;
    } else {
      document.documentElement.dataset.theme = this.mode;
    }
  }

  set(mode: ThemeMode) {
    this.mode = mode;
    if (typeof localStorage !== "undefined") localStorage.setItem(STORAGE_KEY, mode);
    this.applyAttribute();
  }

  cycle() {
    this.set(MODES[(MODES.indexOf(this.mode) + 1) % MODES.length]);
  }

  get label(): string {
    switch (this.mode) {
      case "light":
        return "Clair";
      case "dark":
        return "Sombre";
      default:
        return "Auto";
    }
  }
}

export const theme = new Theme();
