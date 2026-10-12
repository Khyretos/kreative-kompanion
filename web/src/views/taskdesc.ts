/**
 * A task description as guided parts; steps has one step per line, notes any other markdown.
 */
export interface DescParts {
  goal: string;
  steps: string;
  done: string;
  notes: string;
}

export function composeDescription(p: DescParts): string {
  const normalizeLine = (s: string) => s.replace(/\s*\n\s*/g, " ").trim();

  const goalBlock = p.goal.trim() ? "**Goal:** " + normalizeLine(p.goal) : "";
  const doneBlock = p.done.trim() ? "**Done when:** " + normalizeLine(p.done) : "";

  const stepsLines = p.steps.split("\n").map((line) => {
    const markerRegex = /^\s*(\d+[.)]|[-*])\s+/;
    const cleaned = line.replace(markerRegex, "").trim();
    return cleaned.length > 0 ? cleaned : null;
  }).filter((x): x is string => x !== null);

  let stepsBlock = "";
  if (stepsLines.length > 0) {
    stepsBlock = "**Steps**\n" + stepsLines.map((s, i) => `${i + 1}. ${s}`).join("\n");
  }

  const notesBlock = p.notes.trim() ? p.notes.trim() : "";

  const blocks: string[] = [];
  if (goalBlock) blocks.push(goalBlock);
  if (stepsBlock) blocks.push(stepsBlock);
  if (doneBlock) blocks.push(doneBlock);
  if (notesBlock) blocks.push(notesBlock);

  return blocks.length === 0 ? "" : blocks.join("\n\n");
}

/** Splits a stored description back into the parts; whatever does not fit the shape goes to notes. */
export function parseDescription(md: string): DescParts {
  const parts: DescParts = { goal: "", steps: "", done: "", notes: "" };
  const rest: string[] = [];
  const steps: string[] = [];
  const lines = md.replace(/\r\n/g, "\n").split("\n");
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const goal = /^\*\*Goal:\*\*\s*(.*)$/.exec(line);
    const done = /^\*\*Done when:\*\*\s*(.*)$/.exec(line);
    if (goal && !parts.goal) parts.goal = goal[1].trim();
    else if (done && !parts.done) parts.done = done[1].trim();
    else if (/^\*\*Steps\*\*\s*$/.test(line) && !steps.length) {
      while (i + 1 < lines.length && /^\s*\d+[.)]\s*/.test(lines[i + 1])) {
        i++;
        const s = lines[i].replace(/^\s*\d+[.)]\s*/, "").trim();
        if (s) steps.push(s);
      }
    } else rest.push(line);
  }
  parts.steps = steps.join("\n");
  parts.notes = rest.join("\n").trim();
  return parts;
}
