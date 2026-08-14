import { describe, expect, it } from "vitest";
import { currentLabel, freshSteps, PLAN, progressOf, SYNC_KEYS } from "./boot";

describe("plan de démarrage", () => {
  it("porte les étapes du cycle sous les noms rendus par la synchronisation", () => {
    const keys = PLAN.map((step) => step.key);
    for (const key of SYNC_KEYS) expect(keys).toContain(key);
  });

  it("ne nomme jamais deux fois la même étape", () => {
    expect(new Set(PLAN.map((step) => step.key)).size).toBe(PLAN.length);
  });
});

describe("progressOf", () => {
  it("part de zéro quand rien n'a commencé", () => {
    expect(progressOf(freshSteps())).toBe(0);
  });

  it("compte une étape en cours pour moitié", () => {
    const steps = freshSteps();
    steps[0].state = "running";
    expect(progressOf(steps)).toBeCloseTo(0.5 / steps.length);
  });

  it("compte une étape en échec comme faite : elle ne reviendra pas", () => {
    const done = freshSteps().map((step) => ({ ...step, state: "done" as const }));
    const failed = freshSteps().map((step) => ({ ...step, state: "failed" as const }));
    expect(progressOf(done)).toBe(1);
    expect(progressOf(failed)).toBe(1);
  });
});

describe("currentLabel", () => {
  it("annonce l'étape en cours", () => {
    const steps = freshSteps();
    steps[0].state = "done";
    steps[1].state = "running";
    expect(currentLabel(steps)).toBe(PLAN[1].label);
  });

  it("garde la dernière close tant que rien ne tourne", () => {
    const steps = freshSteps();
    steps[0].state = "done";
    steps[1].state = "failed";
    expect(currentLabel(steps)).toBe(PLAN[1].label);
  });

  it("annonce la première étape avant tout travail", () => {
    expect(currentLabel(freshSteps())).toBe(PLAN[0].label);
  });
});
