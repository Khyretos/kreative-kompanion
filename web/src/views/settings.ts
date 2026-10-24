// Models and roles: which model fills which role. Skills belong to roles, so
// swapping a model here keeps everything Kompanion has learned.
import { html, type SafeHtml } from "../core/html";
import type { AppState } from "../state";
import type { AdminSettings, Effort, Role } from "../api/types";
import { EFFORT_LABELS } from "./tasks";
import { pageHead } from "./pagehead";
import { renderAdmin } from "./admin";
import { DEFAULT_STYLE, KINDS, styleOf } from "../core/cardtypes";
import { enabled as deskNotifyOn } from "../core/desknotify";
import { t } from "../core/i18n";

const roleInfo: Record<Role, { name: string; text: string }> = {
  orchestrator: { name: "Orchestrator", text: "Talks with you, plans and splits the work." },
  worker: { name: "Worker", text: "Does the steps: code, files, commands." },
  reviewer: { name: "Reviewer and teacher", text: "Checks results and writes lessons into the skills." },
  overseer: { name: "Overseer", text: "Sees all projects and tasks: status, planning, context for running tasks." },
};

/** OVR-01: the Overseer's name and whether it may add context to running tasks without asking. */
function overseerSection(s: AppState): SafeHtml {
  return html`<section class="overseer-settings">
    <h3 class="label">Overseer</h3>
    <p class="muted small">Turn it on per chat with the eye chip next to the effort. It reads a fresh scan of all your projects and tasks with every answer. Its model is the Overseer role above.</p>
    <div class="field">
      <label for="overseer-name">Name</label>
      <input id="overseer-name" maxlength="40" value="${s.overseer.name}" placeholder="Overseer" title="Shown on its answers; empty uses the server default">
    </div>
    <label><input type="checkbox" id="overseer-interject" ${s.overseer.interject ? "checked" : ""}> May add context to running tasks without asking</label>
    <p class="muted small">Off: it suggests the context and you send it with a button. On: the task reads it before its next step and the project thread shows what was added.</p>
  </section>`;
}

/** AUTO-01: how many tool steps one request may take when Automatic mode is off. */
function stepsSection(s: AppState): SafeHtml {
  return html`<section class="steps-settings">
    <h3 class="label">Steps per request</h3>
    <div class="field">
      <label for="max-steps">Default number of steps</label>
      <input id="max-steps" type="number" min="0" max="100" step="1" value="${s.maxSteps}">
    </div>
    <p class="muted small">How many tool steps one request may take before I stop and ask how to go on (1 to 100, or 0 for no limit). Automatic mode ignores this limit; use Stop to end it at any step.</p>
  </section>`;
}

/** CMD-01: "Explain running actions": on or off, compact or verbose, and the model that explains. */
function explainSection(s: AppState): SafeHtml {
  const e = s.explain;
  const picked = e.providerId && e.modelId ? `${e.providerId}::${e.modelId}` : "";
  return html`<section class="explain-settings">
    <h3 class="label">${t("explainHead")}</h3>
    <label><input type="checkbox" id="explain-on" ${e.on ? "checked" : ""}> ${t("explainHead")}</label>
    <p class="muted small">${t("explainAbout")}</p>
    <fieldset class="checks explain-detail" ${e.on ? "" : "disabled"}>
      <legend>${t("explainDetail")}</legend>
      <label><input type="radio" name="explain-mode" value="compact" ${e.verbose ? "" : "checked"}> ${t("explainCompact")}</label>
      <label><input type="radio" name="explain-mode" value="verbose" ${e.verbose ? "checked" : ""}> ${t("explainVerbose")}</label>
    </fieldset>
    <div class="field">
      <label for="explain-model">${t("explainModel")}</label>
      <select id="explain-model" ${e.on ? "" : "disabled"}>
        <option value="" ${picked ? "" : "selected"}>${t("explainChatModel")}</option>
        ${s.providers.filter((p) => !p.error).map((p) => html`<optgroup label="${p.name}">${p.models.map((m) => html`<option value="${p.id}::${m.id}" ${picked === `${p.id}::${m.id}` ? "selected" : ""}>${m.id}</option>`)}</optgroup>`)}
      </select>
    </div>
    <p class="muted small">${t("explainNote")}</p>
  </section>`;
}


/** NOTIFY: push in this browser: drawn on its own so a state that arrives late never redraws the page. */
export function webPushRow(s: AppState): SafeHtml {
  return html`<div class="web-push-row">
          <button type="button" class="btn" id="web-push-toggle" data-action="web-push-toggle" aria-pressed="${s.webPush === "on" ? "true" : "false"}"
            ${s.webPush === "unsupported" ? "disabled" : ""}>${s.webPush === "on" ? t("wpOff") : t("wpOn")}</button>
          <span class="small ${s.webPush === "on" ? "good-text" : "muted"}" role="status">${t(s.webPush === "on" ? "wpStateOn" : s.webPush === "denied" ? "wpStateDenied" : s.webPush === "unsupported" ? "wpStateNone" : "wpStateOff")}</span>
        </div>`;
}

const DEFAULTS: AdminSettings = {
  appName: "", smtpHost: "", smtpPort: 587, smtpTls: "starttls", smtpUser: "", smtpFrom: "", smtpReplyTo: "",
  colorBrand: "#5c398e", colorLinkDark: "#f3941f", colorLinkLight: "#8f4700", colorAccent: "#f3941f",
};

const LANGS: [string, string][] = [["", "Detect"], ["en", "English"], ["es", "Spanish"], ["nl", "Dutch"]];

/** Item 7: the colour and label of each action type on step and approval cards, saved for this user. */
function cardSection(s: AppState): SafeHtml {
  return html`<section class="card-settings">
    <h3 class="label">Card colours</h3>
    <p class="muted small">Steps and approval cards get a header tinted with this colour and a coloured edge; the text stays bright, whatever colour you pick.</p>
    <ul class="card-kinds">${KINDS.map((k) => {
      const st = styleOf(k, s.cardStyle);
      return html`<li>
        <span class="kind-tag" style="--kind:${st.color}">${st.label}</span>
        <label class="sr-only" for="card-label-${k}">Label for ${DEFAULT_STYLE[k].label}</label>
        <input id="card-label-${k}" data-kind="${k}" class="card-label" value="${st.label}" maxlength="30">
        <label class="sr-only" for="card-color-${k}">Colour for ${DEFAULT_STYLE[k].label}</label>
        <input id="card-color-${k}" data-kind="${k}" class="card-color" type="color" value="${st.color}">
      </li>`;
    })}</ul>
    <button class="btn small" type="button" data-action="card-reset">Back to the brand colours</button>
  </section>`;
}

/** W4: optional voice, per device. Off until switched on here. */
function voiceSection(s: AppState): SafeHtml {
  const p = s.voicePrefs;
  if (!s.voice) return html``;
  return html`<section class="voice-settings">
    <h3 class="label">Voice</h3>
    ${!s.voice.enabled ? html`<p class="muted small">Voice is off on this server.</p>` : html`
      <fieldset class="checks">
        <legend class="sr-only">Voice on this device</legend>
        <label><input type="checkbox" id="voice-input" ${p.input ? "checked" : ""}> Talk with the microphone button</label>
        <label><input type="checkbox" id="voice-read" ${p.readAloud ? "checked" : ""}> Read replies aloud</label>
      </fieldset>
      <div class="field"><label for="voice-lang">I speak</label>
        <select id="voice-lang">${LANGS.map(([v, l]) => html`<option value="${v}" ${v === p.lang ? "selected" : ""}>${l}</option>`)}</select></div>
      <div class="field"><label for="voice-voice">Reading voice</label>
        <select id="voice-voice">${s.voice.voices.map((v) => html`<option value="${v.id}" ${v.id === p.voice ? "selected" : ""}>${v.label}</option>`)}</select></div>
      ${p.lang === "nl" ? html`<p class="warn small">There is no Dutch reading voice yet: replies stay text only while "I speak" is Dutch. The microphone works in Dutch.</p>` : ""}
      <p class="muted small">Speech goes to Whisper and Kokoro on your own server and is never stored. These settings are kept on this device.</p>`}
  </section>`;
}

export function renderSettings(s: AppState): SafeHtml {
  const options = s.providers.flatMap((p) => p.models.map((m) => ({ p, m, value: `${p.id}::${m.id}` })));
  return html`
    <div class="settings-page">
      ${pageHead("Settings", "Your account, models and how Kompanion looks.", "settings-h")}
      ${renderAdmin(s.admin?.settings ?? DEFAULTS, s.admin?.smtpPasswordSet ?? false, s.theme, s.isAdmin && !!s.admin, s.admin?.smtpFromEnv)}
      ${s.notifications ? html`<form class="admin-form" id="notify-form">
        <h3 class="label">Notifications</h3>
        <div class="field">
          <label for="notify-email">Mail me at</label>
          <input type="email" id="notify-email" name="email" value="${s.notifications.email}" placeholder="you@example.com">
        </div>
        <fieldset class="checks">
          <legend>${t("ntfWhen")}</legend>
          <label><input type="checkbox" name="onNeedsInput" ${s.notifications.onNeedsInput ? "checked" : ""}> A task needs me</label>
          <label><input type="checkbox" name="onApproval" ${s.notifications.onApproval !== false ? "checked" : ""}> ${t("ntfApproval")}</label>
          <label><input type="checkbox" name="onFailed" ${s.notifications.onFailed ? "checked" : ""}> A task failed</label>
          <label><input type="checkbox" name="onDone" ${s.notifications.onDone ? "checked" : ""}> A task is done</label>
          <label><input type="checkbox" name="dailySummary" ${s.notifications.dailySummary ? "checked" : ""}> A daily summary (08:00 UTC)</label>
        </fieldset>
        <fieldset class="checks">
          <legend>${t("ntfHow")}</legend>
          <label><input type="checkbox" name="mailOn" ${s.notifications.mailOn !== false ? "checked" : ""}> ${t("ntfMail")}</label>
          <label><input type="checkbox" name="pushOn" ${s.notifications.pushOn !== false ? "checked" : ""}> ${t("ntfPush")}</label>
        </fieldset>
        <fieldset class="quiet-hours">
          <legend>${t("ntfQuiet")}</legend>
          <div class="quiet-row">
            <label for="quiet-from">${t("ntfFrom")}</label><input type="time" id="quiet-from" name="quietFrom" value="${s.notifications.quietFrom ?? ""}">
            <label for="quiet-to">${t("ntfTo")}</label><input type="time" id="quiet-to" name="quietTo" value="${s.notifications.quietTo ?? ""}">
          </div>
          <small class="muted">${t("ntfQuietHelp")}</small>
        </fieldset>
        <p class="muted small">Mails name the task, its state and in one line what is needed, never its description or chat.</p>
        <p id="notify-msg" class="small" role="status"></p>
        <button class="btn primary" type="submit">Save</button>
      </form>` : ""}
      <section class="desk-notify">
        <h3 class="label">On this device</h3>
        <div id="webpush-slot"></div>
        <p class="muted small">${t("wpHelp")}</p>
        <label><input type="checkbox" id="desk-notify" ${deskNotifyOn() ? "checked" : ""}> Desktop notifications when a task needs me, failed or is done, while Kompanion is in the background</label>
        <button type="button" class="btn" id="desk-notify-test" data-action="desk-notify-test">Send test notification</button>
        <p id="desk-notify-msg" class="small muted" role="status"></p>
      </section>
      ${s.isAdmin ? html`<section>
        <h3 class="label">Connections</h3>
        <p>Windshift: <span class="chip ${s.windshift === "connected" ? "good" : ""}">${s.windshift ?? "not configured"}</span></p>
        ${s.windshiftWarning ? html`<p class="warn small" role="status">${s.windshiftWarning}</p>` : ""}
        <p class="muted small">Set in the server's compose file (WINDSHIFT_URL, WINDSHIFT_TOKEN); it can't be changed here.</p>
      </section>` : ""}
      <section>
        <h3 class="label">Roles</h3>
        <p class="muted">Any model can fill any role. Skills and lessons belong to the role, so switching a model keeps them. The effort is the role's default when a task is at Auto; Auto lets Kompanion pick per task.</p>
        <div class="roles">${(Object.keys(roleInfo) as Role[]).map((role) => {
          const current = s.roles.find((r) => r.role === role);
          const value = current ? `${current.providerId}::${current.modelId}` : "";
          const effortValue = current?.effort ?? "auto";
          return html`
            <div class="role">
              <label for="role-${role}"><strong>${roleInfo[role].name}</strong><small>${roleInfo[role].text}</small></label>
              <select id="role-${role}" data-role="${role}">
                ${value ? "" : html`<option value="" selected disabled>Not set</option>`}
                ${options.map((o) => html`<option value="${o.value}" ${o.value === value ? "selected" : ""}>${o.m.id} · ${o.p.name}</option>`)}
              </select>
              <select id="role-effort-${role}" data-role-effort="${role}" aria-label="${roleInfo[role].name}: default effort">
                ${(Object.keys(EFFORT_LABELS) as Effort[]).map((e) => html`<option value="${e}" ${e === effortValue ? "selected" : ""}>${EFFORT_LABELS[e]}</option>`)}
              </select>
            </div>`;
        })}</div>
      </section>
      ${stepsSection(s)}
      ${explainSection(s)}
      ${overseerSection(s)}
      ${voiceSection(s)}
      ${cardSection(s)}
      <div id="providers-slot"></div>
    </div>`;
}
