// OVR-01: the Overseer's proposals under an answer. The server ends an Overseer answer with an
// `:::overseer` line of JSON (tasks to create, context for running tasks); the TASK: and INTERJECT:
// lines the model wrote are hidden from the text. Everything is built with textContent.
export interface ProposedTask { project: string; projectId: string | null; title: string; description: string }
export interface Interjection { taskId: string; title: string; text: string; sent: boolean }
export interface Proposal { tasks: ProposedTask[]; interjections: Interjection[] }

const MARK = ":::overseer";
const LINE = /^\s*(?:[-*] )?\s*(?:\*\*)?(?:TASK|INTERJECT):/i;

const str = (v: unknown): string => (typeof v === "string" ? v : "");

/** The answer without its proposal lines and block (also while streaming), and the proposal. */
export function splitOverseer(text: string): { text: string; proposal: Proposal | null } {
  const i = text.lastIndexOf(MARK);
  const body = (i < 0 ? text : text.slice(0, i)).split("\n").filter((l) => !LINE.test(l)).join("\n").replace(/\n+$/, "");
  if (i < 0) return { text: body, proposal: null };
  try {
    const raw = JSON.parse(text.slice(i + MARK.length).trim().split("\n")[0]) as Record<string, unknown>;
    const tasks = (Array.isArray(raw.tasks) ? raw.tasks : []).flatMap((x): ProposedTask[] => {
      const o = x as Record<string, unknown>;
      return str(o?.title) ? [{ project: str(o.project), projectId: str(o.projectId) || null, title: str(o.title), description: str(o.description) }] : [];
    });
    const interjections = (Array.isArray(raw.interjections) ? raw.interjections : []).flatMap((x): Interjection[] => {
      const o = x as Record<string, unknown>;
      return str(o?.taskId) && str(o.text) ? [{ taskId: str(o.taskId), title: str(o.title), text: str(o.text), sent: o.sent === true }] : [];
    });
    return { text: body, proposal: tasks.length || interjections.length ? { tasks, interjections } : null };
  } catch {
    return { text: body, proposal: null };
  }
}

/** Proposals the user already acted on in this session (message id + group), so a re-render keeps "Created". */
const done = new Set<string>();
export const markDone = (key: string): void => { done.add(key); };

function el<K extends keyof HTMLElementTagNameMap>(tag: K, cls: string, text?: string): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text !== undefined) e.textContent = text;
  return e;
}

/** A finished action: a plain chip in place of its button. */
export function doneChip(text: string): HTMLElement { return el("span", "chip good ov-done", text); }

function createButton(key: string, project: string, list: ProposedTask[]): HTMLButtonElement {
  const btn = el("button", "btn small primary", `Create ${list.length} task${list.length === 1 ? "" : "s"}`);
  btn.type = "button";
  btn.dataset.action = "overseer-create";
  btn.dataset.key = key;
  btn.dataset.json = JSON.stringify({ project, projectId: list[0].projectId, tasks: list.map((t) => ({ title: t.title, description: t.description })) });
  btn.title = project && !list[0].projectId ? `Creates the project ${project} and these tasks` : "Adds these tasks to the project's queue";
  return btn;
}

/** The proposal box: tasks grouped by project with one Create button each, then the context lines. */
export function renderOverseer(p: Proposal, messageId: string): HTMLElement {
  const box = el("div", "overseer-box");
  const groups = new Map<string, ProposedTask[]>();
  for (const t of p.tasks) groups.set(t.project, [...(groups.get(t.project) ?? []), t]);
  for (const [project, list] of groups) {
    const sec = el("section", "ov-group");
    const head = el("div", "ov-head");
    head.append(el("strong", "", project || "No project"));
    if (!list[0].projectId) head.append(el("span", "chip ov-new", "new project"));
    const key = `${messageId}:t:${project}`;
    if (done.has(key)) head.append(doneChip("Created"));
    else head.append(createButton(key, project, list));
    sec.append(head);
    const ul = el("ul", "ov-tasks");
    for (const t of list) {
      const li = el("li", "");
      if (t.description) {
        const d = el("details", "");
        d.append(el("summary", "", t.title), el("p", "muted small", t.description));
        li.append(d);
      } else li.textContent = t.title;
      ul.append(li);
    }
    sec.append(ul);
    box.append(sec);
  }
  p.interjections.forEach((i, n) => {
    const row = el("div", "ov-interject");
    const what = el("p", "small");
    what.append(el("span", "muted", "Context for "), el("strong", "", i.title || "a running task"), document.createTextNode(`: ${i.text}`));
    row.append(what);
    const key = `${messageId}:i:${n}`;
    if (i.sent || done.has(key)) row.append(doneChip("Added to the task"));
    else {
      const btn = el("button", "btn small", "Send to task");
      btn.type = "button";
      btn.dataset.action = "overseer-interject";
      btn.dataset.key = key;
      btn.dataset.task = i.taskId;
      btn.dataset.text = i.text;
      btn.title = "The task reads it before its next step";
      row.append(btn);
    }
    box.append(row);
  });
  return box;
}
