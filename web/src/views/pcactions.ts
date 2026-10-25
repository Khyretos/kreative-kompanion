import { codeSpans, html, type SafeHtml } from "../core/html";
import type { PcAction } from "../api/client";
import { fromTool, renderOutput } from "../core/output";
import { kindOf, styleOf, type CardStyle } from "../core/cardtypes";
import { spawnOf } from "../core/commandcard";
import { t } from "../core/i18n";

export function renderPcPicker(
  machines: { id: string; name: string; online?: boolean }[],
  selected: string | undefined
): SafeHtml {
  if (machines.length === 0) {
    return html``;
  }

  const options = html`
    <option value="">no computer</option>
    ${machines.map((m) => {
      const text = m.online === false ? `${m.name} (offline)` : m.name;
      return html`<option value="${m.id}" ${selected === m.id ? "selected" : ""}>${text}</option>`;
    })}
  `;

  const picked = machines.find((m) => m.id === selected);
  return html`
    <div class="pc-pick">
      <label for="pc-machine">Computer</label>
      <select id="pc-machine" ${picked ? html`aria-describedby="pc-pick-note" title="Kompanion may act on ${picked.name}; every step asks you first."` : ""}>${options}</select>
      ${picked ? html`<span class="sr-only" id="pc-pick-note">Kompanion may act on ${picked.name}; every step asks you first.</span>` : ""}
    </div>
  `;
}

const stateKeys = {
  pending: "stPending",
  approved: "stStarting",
  always: "stStarting",
  granting: "stGranting",
  running: "stRunning",
  denied: "stDenied",
  done: "stDone",
  failed: "stFailed",
  refused: "stRefused",
} as const;
const stateLabel = (state: string): string => (state in stateKeys ? t(stateKeys[state as keyof typeof stateKeys]) : state);

export function renderPcActions(actions: PcAction[], machineNames: Record<string, string>, cardStyle?: CardStyle): SafeHtml {
  if (actions.length === 0) {
    return html``;
  }

  const recent = actions.slice(-6);

  return html`
    <section class="pc-actions" aria-label="Steps on your computer">
      ${recent.map((a) => {
        const label = stateLabel(a.state);
        const need = t(a.state === "pending" ? "needApproval" : "needAutomatic");
        const hasResult = !!a.result;
        const buttons = html`<div class="actions">
          <button class="btn small primary" data-action="pc-decide" data-id="${a.id}" data-decision="approve">${t("approve")}</button>
          <button class="btn small" data-action="pc-decide" data-id="${a.id}" data-decision="always">${t("alwaysAllow")}</button>
          <button class="btn small danger" data-action="pc-decide" data-id="${a.id}" data-decision="deny">${t("deny")}</button>
        </div>`;

        // AGENT-SPAWN: a new chat or task: its prompt, Approve or Deny (no computer, no grant).
        const spawn = spawnOf(a.tool);
        if (spawn) {
          return html`
          <article class="pc-action spawn ${a.state} kind-${kindOf(a.tool)}">
            <header class="card-kind" style="--kind:${styleOf(kindOf(a.tool), cardStyle).color}">
              <span>${t(spawn.kind === "task" ? "spawnTask" : "spawnChat")}</span>
              <span class="need">${need}</span>
            </header>
            <p class="pc-line"><strong>${codeSpans(a.summary)}</strong> <span class="muted small">${t("spawnBy")}</span>${a.state === "pending" ? "" : html` <span class="chip ${a.state}">${label}</span>`}</p>
            <div class="spawn-prompt" role="group" aria-label="${t("spawnPrompt")}">${spawn.prompt}</div>
            ${a.state === "pending" ? html`<div class="actions">
              <button class="btn small primary" data-action="pc-decide" data-id="${a.id}" data-decision="approve">${t("approve")}</button>
              <button class="btn small danger" data-action="pc-decide" data-id="${a.id}" data-decision="deny">${t("deny")}</button></div>` : null}
          </article>`;
        }
        return html`
          <article class="pc-action ${a.state} kind-${kindOf(a.tool)}">
            <header class="card-kind" style="--kind:${styleOf(kindOf(a.tool), cardStyle).color}">
              <span>${styleOf(kindOf(a.tool), cardStyle).label}</span>
              <span class="need">${need}</span>
            </header>
            <p class="pc-line"><strong>${codeSpans(a.summary)}</strong> <span class="muted small">${t("onMachine", { name: machineNames[a.machineId] ?? t("aComputer") })}</span>${a.state === "pending" ? "" : html` <span class="chip ${a.state}">${label}</span>`}</p>
            ${a.needs && a.state === "pending" ? html`<span class="small">${t("needsWhat", { what: a.needs })}</span>` : ""}
            ${hasResult ? renderOutput(fromTool(a.tool, a.result ?? "", machineNames[a.machineId])) : ""}
            ${a.state === "pending" ? buttons : null}
          </article>
        `;
      })}
    </section>
  `;
}
