// Access tab: what each paired computer lets Kompanion do (grants.json on that
// PC, mirrored here), add or revoke a grant, and the history of what was used
// or refused. (gemma4 drafted it; Claude fixed the template and removed inline
// event handlers, which the CSP blocks.)
// ACC-01: times, renew, permanent only after a confirmation, expired grants for a week.
import { html, mount, type SafeHtml } from "../core/html";

export interface GrantView { target: string; rights: string[]; grantedBy: string; grantedAt: string; expires: string | null; pending?: "add" | "revoke" }
export interface AccessEvent { at: string; kind: "granted" | "revoked" | "renewed" | "used" | "refused"; target: string | null; detail: string | null; machine: string }

const when = (iso: string) => new Date(iso).toLocaleString(undefined, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
function left(iso: string, now: number): string { const ms = Date.parse(iso) - now; if (ms < 3_600_000) return `${Math.max(1, Math.round(ms / 60_000))} min`; if (ms < 48 * 3_600_000) return `${Math.round(ms / 3_600_000)} h`; return `${Math.round(ms / 86_400_000)} days`; }
const expired = (g: GrantView, now: number) => !!g.expires && Date.parse(g.expires) <= now;
// Open Renew menus and Expired lists, by data-keep key, so a live re-render keeps them open.
const openKeys = new Set<string>();
document.addEventListener("toggle", (e) => {
  const d = e.target;
  if (d instanceof HTMLDetailsElement && d.dataset.keep) {
    if (d.open) openKeys.add(d.dataset.keep);
    else openKeys.delete(d.dataset.keep);
  }
}, true);
const RENEW: [string, string][] = [["1", "1 hour"], ["24", "1 day"], ["168", "1 week"], ["", "Permanent…"]];
const chips = (g: GrantView) => html`<span class="grant-rights">${g.rights.map((r) => html`<span class="chip ${r === "root" ? "root" : ""}">${r}</span>`)}</span>`;

function grantRow(machineId: string, g: GrantView, now: number): SafeHtml {
  return html`
    <li class="grant-row ${g.pending ? "pending" : ""}">
      <code>${g.target}</code>
      ${chips(g)}
      ${g.pending ? html`<span class="muted small" role="status">${g.pending === "add" ? "waiting for the computer to apply it…" : "revoking, waiting for the computer…"}</span>` : html`<span class="muted small grant-when">by ${g.grantedBy}, ${when(g.grantedAt)} · ${g.expires ? `expires in ${left(g.expires, now)} (${when(g.expires)})` : html`<span class="chip permanent">permanent</span>`}</span>`}
      <details class="grant-renew" data-keep="renew|${machineId}|${g.target}" ${openKeys.has(`renew|${machineId}|${g.target}`) ? "open" : ""}><summary>Renew</summary><div class="grant-renew-list">${RENEW.map(([h, label]) => html`<button type="button" class="btn small" data-action="grant-renew" data-machine="${machineId}" data-target="${g.target}" data-hours="${h}">${label}</button>`)}</div></details>
      <button class="btn small danger" data-action="grant-revoke" data-machine="${machineId}" data-target="${g.target}" ${g.pending ? "disabled" : ""}>Revoke</button>
    </li>`;
}

function expiredRow(machineId: string, g: GrantView): SafeHtml {
  return html`<li class="grant-row expired"><code>${g.target}</code>${chips(g)}<span class="muted small grant-when">ended ${when(g.expires ?? "")}</span><button type="button" class="btn small" data-action="grant-renew" data-machine="${machineId}" data-target="${g.target}" data-hours="24">Grant again</button></li>`;
}

export function grantFromForm(form: HTMLFormElement): { target: string; rights: string[]; expiresHours?: number } {
  const fd = new FormData(form);
  const target = String(fd.get("target") ?? "").trim();
  const rights = Array.from(fd.getAll("rights")).map(String);
  const expiresStr = String(fd.get("expires") ?? "");
  let expiresHours: number | undefined;
  if (expiresStr !== "") {
    expiresHours = Number(expiresStr);
  }
  return { target, rights, expiresHours };
}

export function renderAccess(machines: { id: string; name: string }[], grants: Record<string, GrantView[]>, _history: AccessEvent[]): SafeHtml {
  const now = Date.now();
  const summary = machines.length > 0 ? html`<section class="access-summary" aria-label="All computers"><h3 class="label">All computers</h3><ul>${machines.map((m) => { const a = (grants[m.id] ?? []).filter((g) => !expired(g, now)); return html`<li data-machine="${m.id}"><a href="#access-${m.id}">${m.name}</a> <span class="muted small">${a.length} active · ${a.filter((g) => !g.expires).length} permanent · ${a.filter((g) => g.rights.includes("root")).length} with root</span></li>`; })}</ul></section>` : "";

  return html`
    <div class="task-groups">
      ${summary}
      ${machines.map((m) => {
        const list = grants[m.id] ?? [];
        const active = list.filter((g) => !expired(g, now)).sort((a, b) => (a.expires ? Date.parse(a.expires) : Infinity) - (b.expires ? Date.parse(b.expires) : Infinity));
        const old = list.filter((g) => expired(g, now) && now - Date.parse(g.expires ?? "") <= 7 * 86_400_000).sort((a, b) => Date.parse(b.expires ?? "") - Date.parse(a.expires ?? ""));

        return html`
          <section class="group" id="access-${m.id}" data-machine="${m.id}">
            <h3 class="label">${m.name}</h3>
            ${active.length ? html`<ul class="grants">${active.map((g) => grantRow(m.id, g, now))}</ul>` : html`<p class="muted small">No access granted on this computer.</p>`}
            ${old.length ? html`<details class="grants-expired" data-keep="expired|${m.id}" ${openKeys.has(`expired|${m.id}`) ? "open" : ""}><summary>Expired (${old.length})</summary><ul class="grants-old">${old.map((g) => expiredRow(m.id, g))}</ul></details>` : ""}
            <form class="grant-add" data-machine="${m.id}">
              <input name="target" placeholder="/home/you/projects/app" aria-label="Folder or system" required>
              <span class="grant-rights">
                <label><input type="checkbox" name="rights" value="read" checked> read</label>
                <label><input type="checkbox" name="rights" value="write"> write</label>
                <label><input type="checkbox" name="rights" value="shell"> shell</label>
              </span>
              <select name="expires" aria-label="Expires">
                <option value="1">1 hour</option>
                <option value="24" selected>1 day</option>
                <option value="168">1 week</option>
                <option value="">permanent</option>
              </select>
              <button class="btn small" type="submit">Grant</button>
            </form>
            <form class="grant-add" data-machine="${m.id}">
              <input type="hidden" name="target" value="system">
              <span class="muted small">System rights. Root actions still ask for your password on that PC.</span>
              <span class="grant-rights">
                <label><input type="checkbox" name="rights" value="packages"> packages</label>
                <label><input type="checkbox" name="rights" value="services"> user services</label>
                <label><input type="checkbox" name="rights" value="desktop"> desktop reload</label>
                <label><input type="checkbox" name="rights" value="gpu"> GPU apps (start or stop the studio apps, unload Ollama)</label>
                <label><input type="checkbox" name="rights" value="root"> root (pkexec prompt on the PC)</label>
              </span>
              <select name="expires" aria-label="Expires">
                <option value="1">1 hour</option>
                <option value="24" selected>1 day</option>
                <option value="168">1 week</option>
                <option value="">permanent</option>
              </select>
              <button class="btn small" type="submit">Grant system access</button>
            </form>
          </section>`;
      })}
      <p class="muted small pad">The history of grants and steps is in the Activity tab.</p>
    </div>`;
}

export function confirmPermanent(): Promise<boolean> {
  return new Promise((resolve) => {
    const d = document.createElement("dialog");
    d.className = "perm-dialog";
    d.setAttribute("aria-labelledby", "perm-h");
    mount(d, html`<h2 id="perm-h">Make this permanent?</h2>
      <p>A permanent grant stays until you revoke it. Kompanion can then read, change or delete files (or install packages and run root actions) on this computer at any time. You are responsible for managing it; mistakes can do permanent damage.</p>
      <label class="perm-check"><input type="checkbox" name="understand"> I understand</label>
      <div class="row"><button type="button" class="btn" value="cancel">Cancel</button><button type="button" class="btn primary" value="confirm" disabled>Make permanent</button></div>`);
    const tick = d.querySelector<HTMLInputElement>('input[name="understand"]')!;
    const ok = d.querySelector<HTMLButtonElement>('button[value="confirm"]')!;
    const done = (v: boolean) => { d.close(); d.remove(); resolve(v); };
    tick.addEventListener("change", () => { ok.disabled = !tick.checked; });
    ok.addEventListener("click", () => done(true));
    d.querySelector('button[value="cancel"]')!.addEventListener("click", () => done(false));
    d.addEventListener("cancel", (e) => { e.preventDefault(); done(false); });
    document.body.append(d);
    d.showModal();
  });
}
