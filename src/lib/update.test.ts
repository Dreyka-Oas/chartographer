import { describe, expect, it } from "vitest";
import {
  actionLabel,
  isBusy,
  noteFor,
  ratioOf,
  shouldInstall,
  type UpdateStage,
} from "./update";

const nothing = { received: 0, total: null };

describe("ratioOf", () => {
  it("rend la part reçue quand la taille est connue", () => {
    expect(ratioOf({ received: 0, total: 400 })).toBe(0);
    expect(ratioOf({ received: 100, total: 400 })).toBe(0.25);
    expect(ratioOf({ received: 400, total: 400 })).toBe(1);
  });

  it("ne rend rien tant que la taille manque", () => {
    expect(ratioOf(nothing)).toBeNull();
    expect(ratioOf({ received: 900, total: 0 })).toBeNull();
    expect(ratioOf({ received: 900, total: -1 })).toBeNull();
  });

  it("ne déborde pas quand le serveur a menti sur la taille", () => {
    expect(ratioOf({ received: 900, total: 400 })).toBe(1);
  });
});

describe("isBusy", () => {
  it("ne tient que pendant une opération en cours", () => {
    expect(isBusy("checking")).toBe(true);
    expect(isBusy("downloading")).toBe(true);
    for (const stage of ["idle", "none", "available", "ready", "error"] as UpdateStage[]) {
      expect(isBusy(stage)).toBe(false);
    }
  });
});

describe("shouldInstall", () => {
  it("n'installe que depuis une version trouvée", () => {
    expect(shouldInstall("available")).toBe(true);
    for (const stage of [
      "idle",
      "checking",
      "none",
      "downloading",
      "ready",
      "error",
    ] as UpdateStage[]) {
      expect(shouldInstall(stage)).toBe(false);
    }
  });
});

describe("noteFor", () => {
  it("se tait tant qu'aucune question n'a été posée", () => {
    expect(noteFor("idle", null, nothing, null)).toBe("");
  });

  it("distingue pas encore demandé de déjà à jour", () => {
    expect(noteFor("none", null, nothing, null)).toBe("Cette version est la plus récente.");
  });

  it("nomme la version trouvée", () => {
    expect(noteFor("available", "0.2.0", nothing, null)).toBe("Version 0.2.0 disponible.");
    expect(noteFor("available", null, nothing, null)).toBe("Une nouvelle version est disponible.");
  });

  it("chiffre le téléchargement, avec ou sans taille annoncée", () => {
    expect(noteFor("downloading", "0.2.0", { received: 4_200_000, total: 8_000_000 }, null)).toBe(
      "Téléchargement : 4,2 Mo sur 8,0 Mo.",
    );
    expect(noteFor("downloading", "0.2.0", { received: 1000, total: null }, null)).toBe(
      "Téléchargement : 1,0 ko.",
    );
  });

  it("reprend le message d'erreur, et en fournit un à défaut", () => {
    expect(noteFor("error", null, nothing, "signature refusée")).toBe("signature refusée");
    expect(noteFor("error", null, nothing, null)).toBe("Mise à jour impossible.");
  });
});

describe("actionLabel", () => {
  it("propose d'installer une fois la version connue", () => {
    expect(actionLabel("available", "0.2.0")).toBe("Installer 0.2.0");
    expect(actionLabel("available", null)).toBe("Installer");
  });

  it("dit ce qui se passe pendant une opération", () => {
    expect(actionLabel("checking", null)).toBe("Recherche…");
    expect(actionLabel("downloading", "0.2.0")).toBe("Téléchargement…");
  });

  it("repose la question partout ailleurs", () => {
    for (const stage of ["idle", "none", "ready", "error"] as UpdateStage[]) {
      expect(actionLabel(stage, "0.2.0")).toBe("Vérifier maintenant");
    }
  });
});
