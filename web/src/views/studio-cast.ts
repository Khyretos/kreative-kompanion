// STU-N2: the Studio Cast panel's state (two to four people, left to right). Pure functions, tested in tests-node.
export const CAST_MIN = 2;
export const CAST_MAX = 4;

export function castNew(): string[] {
  return ["", ""];
}

export function castAdd(p: string[]): string[] {
  if (p.length < CAST_MAX) {
    return [...p, ""];
  }
  return p;
}

export function castRemove(p: string[], i: number): string[] {
  if (i < 0 || i >= p.length) {
    return p;
  }
  if (p.length - 1 < CAST_MIN) {
    return p;
  }
  return [...p.slice(0, i), ...p.slice(i + 1)];
}

export function castMove(p: string[], i: number, dir: -1 | 1): string[] {
  const j = i + dir;
  if (j < 0 || j >= p.length) {
    return p;
  }
  const copy = [...p];
  [copy[i], copy[j]] = [copy[j], copy[i]];
  return copy;
}

export function castType(n: number): string {
  return `vn-cast-${n}`;
}

export function castParams(p: string[]): Record<string, string> {
  const params: Record<string, string> = {};
  for (let i = 0; i < p.length; i++) {
    params[`person${i + 1}`] = p[i].trim();
  }
  return params;
}

export function castProblem(scene: string, p: string[]): string {
  if (!scene.trim()) {
    return "Describe the place and mood.";
  }
  for (let i = 0; i < p.length; i++) {
    if (!p[i].trim()) {
      return `Describe person ${i + 1}.`;
    }
  }
  return "";
}
