import { describe, expect, it } from "vitest";
import { compactNumber, countryLabel, deltaPercent, formatDay, formatMoney } from "./format";

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

describe("countryLabel", () => {
  it("traduit un code ISO et isole l'inconnu", () => {
    expect(countryLabel("??")).toBe("Inconnu");
    expect(countryLabel("DE")).toBe("Allemagne");
  });
});
