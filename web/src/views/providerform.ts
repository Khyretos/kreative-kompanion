// PROV-UI: model providers in Settings. Every provider shows its models; admins add, change and
// remove their own (those from kompanion.toml stay as they are). A key is never shown or sent: the
// form names the environment variable or the secret file that holds it.
import { html, type SafeHtml } from "../core/html";
import { t } from "../core/i18n";
import type { CustomProvider, ProviderDraft } from "../api/types";
import type { AppState } from "../state";
import { icon } from "./icons";

/** Ready-made settings for common providers; the key's place is a suggestion. */
export const PRESETS: { id: string; draft: ProviderDraft }[] = [
  { id: "xai", draft: { name: "xAI Grok", kind: "openai-compatible", baseUrl: "https://api.x.ai/v1", local: false, keySource: "env", apiKeyEnv: "XAI_API_KEY", apiKeyFile: "", maxTokens: "", contextWindow: "" } },
  { id: "openai", draft: { name: "OpenAI", kind: "openai-compatible", baseUrl: "https://api.openai.com/v1", local: false, keySource: "env", apiKeyEnv: "OPENAI_API_KEY", apiKeyFile: "", maxTokens: "", contextWindow: "" } },
  { id: "anthropic", draft: { name: "Claude (Anthropic API)", kind: "anthropic", baseUrl: "https://api.anthropic.com", local: false, keySource: "env", apiKeyEnv: "ANTHROPIC_API_KEY", apiKeyFile: "", maxTokens: "", contextWindow: "" } },
  { id: "deepseek", draft: { name: "DeepSeek", kind: "openai-compatible", baseUrl: "https://api.deepseek.com", local: false, keySource: "env", apiKeyEnv: "DEEPSEEK_API_KEY", apiKeyFile: "", maxTokens: "", contextWindow: "" } },
  { id: "openrouter", draft: { name: "OpenRouter", kind: "openai-compatible", baseUrl: "https://openrouter.ai/api/v1", local: false, keySource: "env", apiKeyEnv: "OPENROUTER_API_KEY", apiKeyFile: "", maxTokens: "", contextWindow: "" } },
  { id: "ollama", draft: { name: "Ollama", kind: "openai-compatible", baseUrl: "http://ollama:11434/v1", local: true, keySource: "none", apiKeyEnv: "", apiKeyFile: "", maxTokens: "", contextWindow: "" } },
  { id: "other", draft: { name: "", kind: "openai-compatible", baseUrl: "https://", local: false, keySource: "env", apiKeyEnv: "", apiKeyFile: "", maxTokens: "", contextWindow: "" } },
];

const PRESET_LABEL: Record<string, string> = {
  xai: "xAI Grok", openai: "OpenAI", anthropic: "Anthropic (Claude)", deepseek: "DeepSeek", openrouter: "OpenRouter", ollama: "Ollama (local)",
};

/** The form's draft for a provider that exists. */
export function draftOf(p: CustomProvider): ProviderDraft {
  return {
    id: p.id, name: p.name, kind: p.kind, baseUrl: p.baseUrl, local: p.local,
    keySource: p.apiKeyEnv ? "env" : p.apiKeyFile ? "file" : "none",
    apiKeyEnv: p.apiKeyEnv ?? "", apiKeyFile: p.apiKeyFile ?? "",
    maxTokens: p.maxTokens ? String(p.maxTokens) : "", contextWindow: p.contextWindow ? String(p.contextWindow) : "",
  };
}

/** Where a provider's key comes from, in words (never the key). */
function keyNote(p: CustomProvider): string {
  const where = p.apiKeyEnv ? t("provKeyFromEnv", { name: p.apiKeyEnv }) : p.apiKeyFile ? t("provKeyFromFile", { path: p.apiKeyFile }) : t("provNoKey");
  return p.apiKeyEnv || p.apiKeyFile ? `${where} · ${p.hasKey ? t("provKeyFound") : t("provKeyMissing")}` : where;
}

function providerForm(d: ProviderDraft, error: string | undefined, dirs: string[]): SafeHtml {
  const isNew = !d.id;
  return html`<form class="provider-form" id="provider-form" aria-labelledby="provider-form-h">
    <h4 id="provider-form-h">${isNew ? t("provAdd") : t("provEdit", { name: d.name })}</h4>
    ${isNew ? html`<div class="field">
      <label for="prov-preset">${t("provPreset")}</label>
      <select id="prov-preset">
        <option value="" selected>${t("provPresetPick")}</option>
        ${PRESETS.map((p) => html`<option value="${p.id}">${PRESET_LABEL[p.id] ?? t("provPresetOther")}</option>`)}
      </select>
    </div>` : ""}
    <div class="field"><label for="prov-name">${t("provName")}</label><input id="prov-name" name="name" maxlength="60" required value="${d.name}"></div>
    <div class="field"><label for="prov-kind">${t("provKind")}</label>
      <select id="prov-kind" name="kind">
        <option value="openai-compatible" ${d.kind === "openai-compatible" ? "selected" : ""}>${t("provKindOpenai")}</option>
        <option value="anthropic" ${d.kind === "anthropic" ? "selected" : ""}>${t("provKindAnthropic")}</option>
      </select></div>
    <div class="field"><label for="prov-url">${t("provUrl")}</label><input id="prov-url" name="baseUrl" type="url" required value="${d.baseUrl}" inputmode="url" spellcheck="false"></div>
    <label class="check"><input type="checkbox" name="local" ${d.local ? "checked" : ""}> ${t("provLocal")}</label>
    <fieldset class="checks prov-key">
      <legend>${t("provKey")}</legend>
      <label><input type="radio" name="keySource" value="env" ${d.keySource === "env" ? "checked" : ""}> ${t("provKeyEnv")}</label>
      <label><input type="radio" name="keySource" value="file" ${d.keySource === "file" ? "checked" : ""}> ${t("provKeyFile")}</label>
      <label><input type="radio" name="keySource" value="none" ${d.keySource === "none" ? "checked" : ""}> ${t("provKeyNone")}</label>
    </fieldset>
    ${d.keySource === "env" ? html`<div class="field"><label for="prov-env">${t("provEnvName")}</label>
      <input id="prov-env" name="apiKeyEnv" value="${d.apiKeyEnv}" placeholder="XAI_API_KEY" autocomplete="off" spellcheck="false">
      <small class="muted">${t("provEnvHelp")}</small></div>` : ""}
    ${d.keySource === "file" ? html`<div class="field"><label for="prov-file">${t("provFilePath")}</label>
      <input id="prov-file" name="apiKeyFile" value="${d.apiKeyFile}" placeholder="${dirs[0] ?? "/run/secrets"}/xai_api_key" autocomplete="off" spellcheck="false">
      <small class="muted">${t("provFileHelp", { dirs: dirs.join(" · ") })}</small></div>` : ""}
    <div class="prov-numbers">
      <div class="field"><label for="prov-max">${t("provMaxTokens")}</label><input id="prov-max" name="maxTokens" type="number" min="1" max="200000" value="${d.maxTokens}" placeholder="${t("provDefault")}"></div>
      <div class="field"><label for="prov-window">${t("provWindow")}</label><input id="prov-window" name="contextWindow" type="number" min="1000" max="10000000" value="${d.contextWindow}" placeholder="${t("provLookUp")}"></div>
    </div>
    <p class="muted small">${t("provNeverShown")}</p>
    ${error ? html`<p class="error small" role="alert">${error}</p>` : ""}
    <div class="form-buttons">
      <button class="btn" type="button" data-action="provider-cancel">${t("msgCancel")}</button>
      <button class="btn primary" type="submit">${isNew ? t("provAddButton") : t("provSave")}</button>
    </div>
  </form>`;
}

/** The "Connected models" section: every provider with its models; an admin's own ones editable. */
export function providersSection(s: AppState): SafeHtml {
  const custom = new Map((s.customProviders?.providers ?? []).map((p) => [p.id, p]));
  const form = s.providerForm;
  return html`<section class="providers-settings">
    <h3 class="label">${t("provHead")}</h3>
    <ul class="providers">${s.providers.map((p) => {
      const own = custom.get(p.id);
      return html`
      <li class="provider" data-provider="${p.id}">
        <div class="provider-head">
          <strong>${p.name}</strong>
          <span class="chip ${p.local ? "good" : ""}">${p.local ? "Local" : "Cloud"}</span>
          ${s.isAdmin && own ? html`<span class="provider-tools">
            <button class="btn small" id="prov-edit-${p.id}" type="button" data-action="provider-edit" data-id="${p.id}">${icon("edit")}${t("provEditButton")}</button>
            <button class="btn small danger" type="button" data-action="provider-remove" data-id="${p.id}">${icon("trash")}${t("provRemove")}</button>
          </span>` : ""}
        </div>
        <small class="muted">${p.kind === "anthropic" ? "Anthropic API" : "OpenAI-compatible API"} · ${p.baseUrl}</small>
        ${own ? html`<small class="muted">${keyNote(own)}</small>` : s.isAdmin && s.customProviders ? html`<small class="muted">${t("provFromToml")}</small>` : ""}
        ${p.error ? html`<p class="error small">Can't reach it: ${p.error}</p>` : ""}
        ${form?.id === p.id ? providerForm(form, s.providerFormError, s.customProviders?.secretDirs ?? []) : ""}
        ${p.models.map((m) => html`
          <div class="model-row">
            <code>${m.id}</code>
            <span class="chips">
              ${m.contextTokens ? html`<span class="chip">${Math.round(m.contextTokens / 1024)}k context</span>` : ""}
              ${m.tokensPerSecond ? html`<span class="chip">${m.tokensPerSecond} tok/s</span>` : ""}
              ${m.toolCalls === undefined ? html`<span class="chip">not tested yet</span>` : html`
                <span class="chip ${m.toolCalls ? "good" : "bad"}">tool calls ${m.toolCalls ? "ok" : "no"}</span>
                <span class="chip ${m.jsonSchema ? "good" : "bad"}">JSON schema ${m.jsonSchema ? "ok" : "no"}</span>`}
            </span>
          </div>`)}
      </li>`;
    })}</ul>
    ${form && !form.id ? providerForm(form, s.providerFormError, s.customProviders?.secretDirs ?? []) : ""}
    ${s.isAdmin && !form ? html`<button class="btn" id="add-provider" type="button" data-action="add-provider">${icon("plus")} ${t("provAdd")}</button>` : ""}
    ${s.isAdmin ? "" : html`<p class="muted small">${t("provAdminOnly")}</p>`}
  </section>`;
}
