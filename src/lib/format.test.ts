import { describe, expect, it } from "vitest";
import {
  compactNumber,
  countryLabel,
  deltaPercent,
  formatAge,
  formatBytes,
  formatDay,
  formatDayLong,
  formatMonth,
  formatMoney,
  formatRange,
  lastDayOfMonth,
} from "./format";

const plain = (value: string) => value.replace(/\s/g, " ");

describe("compactNumber", () => {
  it("abrège au-delà du millier", () => {
    expect(compactNumber(999)).toBe("999");
    expect(plain(compactNumber(1776))).toBe("1,8 k");
    expect(plain(compactNumber(176968))).toBe("177,0 k");
    expect(plain(compactNumber(2_400_000))).toBe("2,4 M");
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
    expect(plain(formatMoney("0.00762273691987854525"))).toBe("0,01 $");
    expect(plain(formatMoney("12.5"))).toBe("12,50 $");
    expect(plain(formatMoney("nope"))).toBe("0,00 $");
  });
});

describe("formatDay", () => {
  it("rend un jour ISO en jour court", () => {
    expect(formatDay("2026-08-11")).toBe("11 août");
  });
});

describe("formatDayLong", () => {
  it("ajoute l'année au jour court", () => {
    expect(formatDayLong("2026-08-11")).toBe("11 août 2026");
  });
});

describe("formatMonth", () => {
  it("écrit le mois en toutes lettres", () => {
    expect(formatMonth("2026-08")).toBe("août 2026");
    expect(formatMonth("2026-02")).toBe("févr. 2026");
  });
});

describe("lastDayOfMonth", () => {
  it("trouve la fin de mois, années bissextiles comprises", () => {
    expect(lastDayOfMonth("2026-08")).toBe("2026-08-31");
    expect(lastDayOfMonth("2026-02")).toBe("2026-02-28");
    expect(lastDayOfMonth("2024-02")).toBe("2024-02-29");
    expect(lastDayOfMonth("2026-12")).toBe("2026-12-31");
  });
});

describe("formatRange", () => {
  it("décrit une fenêtre bornes incluses", () => {
    expect(formatRange("2026-08-01", "2026-08-31")).toBe("1 août → 31 août 2026");
  });
});

describe("formatBytes", () => {
  it("monte d'unité par milliers", () => {
    expect(formatBytes(0)).toBe("0 o");
    expect(formatBytes(999)).toBe("999 o");
    expect(formatBytes(1000)).toBe("1,0 ko");
    expect(formatBytes(4_200_000)).toBe("4,2 Mo");
    expect(formatBytes(2_500_000_000)).toBe("2,5 Go");
  });

  it("refuse ce qui n'est pas une taille", () => {
    expect(formatBytes(-1)).toBe("—");
    expect(formatBytes(Number.NaN)).toBe("—");
  });
});

describe("formatAge", () => {
  it("échelonne l'ancienneté", () => {
    expect(formatAge(null)).toBe("jamais");
    expect(formatAge(30_000)).toBe("à l'instant");
    expect(formatAge(25 * 60_000)).toBe("il y a 25 min");
    expect(formatAge(3 * 3_600_000)).toBe("il y a 3 h");
    expect(formatAge(50 * 3_600_000)).toBe("il y a 2 j");
  });
});

describe("countryLabel", () => {
  it("traduit un code ISO et isole l'inconnu", () => {
    expect(countryLabel("??")).toBe("Inconnu");
    expect(countryLabel("DE")).toBe("Allemagne");
  });
});
