// PROJ-OVERVIEW: the Overview at the top of a project's Tasks pane: what it is, where it lives, what it may
// reach, how far along it is, what happened lately and which models worked on it. Glance first (header,
// four tiles, four lines of About), the rest in sections; the owner edits name, description, about and
// links in place. The data comes from one request (GET /api/projects/{id}/overview).
import { html, type SafeHtml } from "../core/html";
import { relTime } from "../core/time";
import type { Project, ProjectLink, ProjectOverview } from "../api/types";
import { icon } from "./icons";

export type SectionName = "where" | "access" | "build" | "activity" | "ai" | "settings";

export interface OverviewView {
  project: Project; // from the project list: shown while the overview loads
  o: ProjectOverview | undefined; // undefined while loading
  editing: boolean; // the edit form is open
  more: boolean; // About is shown in full
  activityAll: boolean; // Recent activity shows 10, not 5
  preview: boolean; // the edit form shows the About preview
  open: (sec: SectionName) => boolean; // whether a section is expanded
}

const TYPE_LABEL: Record<string, string> = { chat: "Chat", game: "Game", programming: "Programming" };

/** The links field of the edit form: one link per line, "Label https://address". */
export function parseLinks(text: string): ProjectLink[] {
  return [];
}

/** The links as the edit form shows them, one per line. */
export function linksText(links: ProjectLink[]): string {
  return "";
}

/** A grant's rights in words: "Read and write", "Read only", "Run commands", ... */
export function rightsLabel(rights: string[]): string {
  return "";
}

/** A collapsible section with a header button. `count` is shown next to the title when given. */
export function section(name: SectionName, title: string, count: number | undefined, open: boolean, body: SafeHtml): SafeHtml {
  return html``;
}

function header(v: OverviewView): SafeHtml {
  return html``;
}

function tiles(v: OverviewView): SafeHtml {
  return html``;
}

function about(v: OverviewView): SafeHtml {
  return html``;
}

function where(v: OverviewView): SafeHtml {
  return html``;
}

function access(v: OverviewView): SafeHtml {
  return html``;
}

function build(v: OverviewView): SafeHtml {
  return html``;
}

function activity(v: OverviewView): SafeHtml {
  return html``;
}

function ai(v: OverviewView): SafeHtml {
  return html``;
}

function editForm(v: OverviewView): SafeHtml {
  return html``;
}

/** The whole Overview; `settings` is the project's settings panel, shown inside the Settings section. */
export function renderOverview(v: OverviewView, settings: SafeHtml): SafeHtml {
  return html``;
}
