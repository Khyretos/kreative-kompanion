// Tiny HTML templating. Every interpolated value is escaped unless it is
// already SafeHtml (the result of another html`` call). Untrusted rich text
// (model markdown) never goes through here; it uses core/markdown.ts.

export class SafeHtml {
  constructor(readonly value: string) {}
  toString(): string {
    return this.value;
  }
}

type Value = SafeHtml | string | number | boolean | null | undefined | Value[];

export function esc(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function render(value: Value): string {
  if (value === null || value === undefined || value === false) return "";
  if (Array.isArray(value)) return value.map(render).join("");
  if (value instanceof SafeHtml) return value.value;
  return esc(String(value));
}

export function html(strings: TemplateStringsArray, ...values: Value[]): SafeHtml {
  let out = strings[0];
  values.forEach((v, i) => {
    out += render(v) + strings[i + 1];
  });
  return new SafeHtml(out);
}

/** Plain text where `backticked` parts become <code> (step summaries such as "Read `Cargo.toml`"). */
export function codeSpans(text: string): SafeHtml {
  const parts = text.split(/`([^`\n]+)`/);
  return html`${parts.map((p, i) => (i % 2 ? html`<code>${p}</code>` : p))}`;
}

// Trusted Types: with the CSP `require-trusted-types-for 'script'`, the
// browser refuses raw strings in innerHTML. Our templates are escaped by
// construction, so this policy is the one place that vouches for them.
type TTPolicy = { createHTML(s: string): unknown };
const tt = (globalThis as { trustedTypes?: { createPolicy(n: string, r: { createHTML(s: string): string }): TTPolicy } }).trustedTypes;
const policy: TTPolicy | undefined = tt?.createPolicy("app", {
  createHTML: (s) => s,
  // The one script URL the app loads itself: its service worker (Apps 2).
  createScriptURL: (s: string) => { if (s !== "sw.js") throw new Error(`script URL not allowed: ${s}`); return s; },
} as { createHTML(s: string): string });

/** The service worker's URL as a TrustedScriptURL where Trusted Types are on. */
export function swUrl(): string {
  const p = policy as unknown as { createScriptURL?(s: string): string } | undefined;
  return p?.createScriptURL ? p.createScriptURL("sw.js") : "sw.js";
}

/** Replace an element's content with a template. */
export function mount(el: Element, content: SafeHtml): void {
  (el as { innerHTML: unknown }).innerHTML = policy ? policy.createHTML(content.value) : content.value;
}

/** Build a detached element from a template with one root element. */
export function fragment(content: SafeHtml): HTMLElement {
  const t = document.createElement("template");
  mount(t, content);
  return t.content.firstElementChild as HTMLElement;
}

export function $(selector: string, root: ParentNode = document): HTMLElement {
  const el = root.querySelector<HTMLElement>(selector);
  if (!el) throw new Error(`Missing element: ${selector}`);
  return el;
}

// Actions still running, by a key built from the element's data attributes, so
// the busy state survives a re-render that replaces the element.
const busy = new Set<string>();
const busyKey = (el: HTMLElement) => JSON.stringify(Object.entries(el.dataset).sort());

function markBusy(el: HTMLElement, on: boolean): void {
  if (on) el.setAttribute("aria-busy", "true");
  else el.removeAttribute("aria-busy");
  if (el instanceof HTMLButtonElement) el.disabled = on;
}

/**
 * Event delegation: one listener on `root` handles clicks on any element with
 * data-action="name", now or added later by a re-render.
 */
export function onAction(
  root: HTMLElement,
  handlers: Record<string, (el: HTMLElement, ev: Event) => void | Promise<unknown>>,
): void {
  const run = (el: HTMLElement, ev: Event): void => {
    if (!root.contains(el) || el.getAttribute("aria-busy") === "true") return;
    const handler = handlers[el.dataset.action ?? ""];
    if (!handler) return;
    ev.preventDefault();
    const result = handler(el, ev);
    if (!(result instanceof Promise)) return;
    const key = busyKey(el);
    busy.add(key);
    markBusy(el, true);
    result.catch(() => undefined).finally(() => {
      busy.delete(key);
      for (const x of root.querySelectorAll<HTMLElement>("[aria-busy]")) if (busyKey(x) === key) markBusy(x, false);
    });
  };
  // A live re-render (a step landing, a timer) can replace the element between press and
  // release; the browser then sends no click. Run the action on release when the element
  // under the pointer is the re-rendered twin of the one pressed (same action, id and data-id).
  const twin = (el: HTMLElement) => `${el.dataset.action}|${el.id}|${el.dataset.id ?? ""}`;
  let pressed: { el: HTMLElement; key: string } | null = null;
  let ranOnRelease: string | null = null; // Chrome may still send a click to the twin: run once
  root.addEventListener("pointerdown", (ev) => {
    ranOnRelease = null;
    const el = (ev.target as HTMLElement).closest<HTMLElement>("[data-action]");
    pressed = el && ev.button === 0 ? { el, key: twin(el) } : null;
  });
  root.addEventListener("pointerup", (ev) => {
    const p = pressed;
    pressed = null;
    if (!p || p.el.isConnected) return; // still there: the browser sends the click itself
    const el = (ev.target as HTMLElement).closest<HTMLElement>("[data-action]");
    if (el && twin(el) === p.key) {
      ranOnRelease = p.key;
      run(el, ev);
    }
  });
  root.addEventListener("click", (ev) => {
    const el = (ev.target as HTMLElement).closest<HTMLElement>("[data-action]");
    if (el && ranOnRelease && twin(el) === ranOnRelease) {
      ranOnRelease = null;
      ev.preventDefault(); // already done on release (and no native <summary> toggle)
      return;
    }
    if (el) run(el, ev);
  });
}

/** Call after re-rendering: marks the new elements of actions that are still running. */
export function restoreBusy(root: HTMLElement): void {
  if (busy.size === 0) return;
  for (const el of root.querySelectorAll<HTMLElement>("[data-action]")) if (busy.has(busyKey(el))) markBusy(el, true);
}

/** While `work` runs, the form's buttons are disabled and its submit button shows
 *  the busy spinner (worker/web lesson 11). Returns `work`. */
export function busyWhile<T>(form: HTMLFormElement, work: Promise<T>): Promise<T> {
  const buttons = [...form.querySelectorAll<HTMLButtonElement>("button")];
  for (const b of buttons) {
    b.disabled = true;
    if (b.type === "submit") b.setAttribute("aria-busy", "true");
  }
  const done = () => {
    for (const b of buttons) {
      b.disabled = false;
      b.removeAttribute("aria-busy");
    }
  };
  work.then(done, done);
  return work;
}

/**
 * Update an element's content to a template in place: nodes that stay the same are kept, so a
 * live re-render no longer replays entry animations, reloads pictures or closes what is open
 * (the "flashing" sidebar and project panel). Fields keep what the user changed: a value or a
 * tick is only set when the template's own value for it changed.
 */
export function morph(el: Element, content: SafeHtml): void {
  const t = document.createElement("template");
  mount(t, content);
  morphChildren(el, t.content);
}

function sameKind(a: Node, b: Node): boolean {
  if (a.nodeType !== b.nodeType || a.nodeName !== b.nodeName) return false;
  if (a instanceof Element && b instanceof Element) {
    // A different id or item key is a different thing: replace it rather than reshape it.
    if (a.id !== b.id || a.getAttribute("data-id") !== b.getAttribute("data-id")) return false;
    if (a.nodeName === "INPUT" && (a as HTMLInputElement).type !== (b as HTMLInputElement).type) return false;
  }
  return true;
}

function morphChildren(from: Node, to: Node): void {
  let a = from.firstChild;
  let b = to.firstChild;
  while (b) {
    const nextB = b.nextSibling;
    if (a && sameKind(a, b)) {
      morphNode(a, b);
      a = a.nextSibling;
    } else if (a) {
      const keep = a.nextSibling;
      from.replaceChild(b, a);
      a = keep;
    } else {
      from.appendChild(b);
    }
    b = nextB;
  }
  while (a) {
    const next = a.nextSibling;
    from.removeChild(a);
    a = next;
  }
}

// The pick each kept select last had in its template.
const picks = new WeakMap<HTMLSelectElement, number>();

function morphNode(a: Node, b: Node): void {
  if (!(a instanceof Element) || !(b instanceof Element)) {
    if (a.nodeValue !== b.nodeValue) a.nodeValue = b.nodeValue;
    return;
  }
  for (const { name, value } of [...b.attributes]) {
    if (a.getAttribute(name) === value) continue;
    a.setAttribute(name, value);
    if (a instanceof HTMLInputElement && name === "value") a.value = value;
    if (a instanceof HTMLInputElement && name === "checked") a.checked = true;
  }
  for (const { name } of [...a.attributes]) {
    if (b.hasAttribute(name)) continue;
    a.removeAttribute(name);
    if (a instanceof HTMLInputElement && name === "checked") a.checked = false;
    if (a instanceof HTMLDetailsElement && name === "open") a.open = false;
  }
  if (a instanceof HTMLTextAreaElement) {
    if (a.defaultValue !== (b as HTMLTextAreaElement).defaultValue) a.value = a.defaultValue = (b as HTMLTextAreaElement).defaultValue;
    return;
  }
  morphChildren(a, b);
  // A select's pick follows the template only when the template's pick changed.
  if (a instanceof HTMLSelectElement) {
    const want = [...(b as HTMLSelectElement).options].findIndex((o) => o.defaultSelected);
    if (picks.get(a) !== want) { picks.set(a, want); if (want >= 0) a.selectedIndex = want; }
  }
}
