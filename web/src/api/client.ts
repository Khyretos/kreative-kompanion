// The app talks to a Kompanion server only through this interface.
// `MockApi` implements it for the draft; `HttpApi` (next milestone) will call
// the real server over HTTPS + server-sent events.
import type { NotificationPrefs, TaskState, AdminSettings, ThemeChoice, DaySummary, MachineStats, ServerStatus, Chat, Message, Project, RoleAssignment, ModelProvider, SearchResult, Server, Task } from "./types";

/** A lesson a task run's reviewer proposed in the project thread (SK-02). */
export interface Lesson {
  id: string;
  chatId: string;
  /** The skill card it would go into, e.g. "worker/rust/SKILL". */
  card: string;
  text: string;
  /** The review finding it comes from. */
  finding: string;
  /** general (for any project, published to kompas-skills) or private (this setup only). */
  layer: "general" | "private";
  /** The model that proposed it. */
  proposedBy: string;
  state: "proposed" | "accepted" | "dismissed";
  createdAt: string;
}

/** A step Kompanion wants to run on a paired computer (F6), waiting for approval. */
export interface PcAction {
  id: string;
  machineId: string;
  summary: string;
  state: "pending" | "approved" | "always" | "granting" | "running" | "denied" | "done" | "failed" | "refused" | "stopped";
  /** The grant the step needs, e.g. "packages + root (asks for the password on the PC)". */
  needs?: string | null;
  /** The runner tool call: `{ "tool": "shell", "cwd": .., "command": .. }` and so on. */
  tool?: Record<string, unknown>;
  result: string | null;
  createdAt: string;
  /** When it started running (approved, or created for automatic steps). */
  startedAt?: string;
}

export interface KompanionApi {
  discover(): Promise<Server[]>;
  connect(url: string): Promise<Server>;
  status(): Promise<ServerStatus>;
  setup(code: string, name: string, password: string): Promise<void>;
  login(name: string, password: string): Promise<void>;
  /** Signs out; returns the provider's sign-out page when single sign-on was used. */
  logout(): Promise<string | null>;
  setTheme(theme: ThemeChoice): Promise<void>;
  setMachinesRefresh(seconds: number): Promise<void>;
  setGpuPins(pins: string[]): Promise<void>;
  /** Card colours and labels per action type (item 7), saved for this user. */
  setCardStyle(style: import("../core/cardtypes").CardStyle): Promise<void>;
  getNotifications(): Promise<NotificationPrefs>;
  setNotifications(p: NotificationPrefs): Promise<void>;
  /** Keeps the live machine feed on for about 15 s. */
  watchMachines(): Promise<void>;
  /** Pairs a PC; the token is returned only this once. */
  pairMachine(name: string): Promise<{ id: string; name: string; token: string }>;
  /** A one-time code for the one-line runner install (15 minutes). */
  pairCode(name: string): Promise<{ code: string; expiresAt: string }>;
  unpairMachine(id: string): Promise<void>;
  listGrants(machineId: string): Promise<import("../views/access").GrantView[]>;
  addGrant(machineId: string, target: string, rights: string[], expiresHours?: number): Promise<void>;
  revokeGrant(machineId: string, target: string): Promise<void>;
  accessHistory(): Promise<import("../views/access").AccessEvent[]>;
  getAdmin(): Promise<{ settings: AdminSettings; smtpPasswordSet: boolean }>;
  saveAdmin(settings: AdminSettings): Promise<AdminSettings>;
  testMail(to: string): Promise<void>;
  uploadLogo(file: File): Promise<void>;
  removeLogo(): Promise<void>;

  listProjects(): Promise<Project[]>;
  /** A project's type, and for programming projects its repo folder and computer ("" clears). */
  setProjectSettings(projectId: string, change: { type?: Project["type"]; repoFolder?: string; repoMachineId?: string }): Promise<void>;
  /** Library assets attached to a game project. */
  projectAssets(projectId: string): Promise<import("../views/projectpanel").ProjectAsset[]>;
  attachAsset(projectId: string, assetId: number): Promise<void>;
  detachAsset(projectId: string, assetId: number): Promise<void>;
  listChats(): Promise<Chat[]>;
  listMessages(chatId: string): Promise<Message[]>;
  /** Global search over the user's tasks, chats, messages and projects. */
  search(q: string): Promise<SearchResult[]>;
  listTasks(projectId?: string): Promise<Task[]>;
  listProviders(): Promise<ModelProvider[]>;
  listMachines(): Promise<MachineStats[]>;
  today(): Promise<DaySummary>;
  listRoles(projectId?: string): Promise<RoleAssignment[]>;
  setRole(assignment: RoleAssignment, projectId?: string): Promise<void>;

  createChat(title: string, projectId?: string): Promise<Chat>;
  /** The project's thread chat (created on first use); returns its chat id. */
  openThread(projectId: string): Promise<string>;
  updateChat(chatId: string, change: { title?: string; pinned?: boolean; archived?: boolean; projectId?: string; effort?: import("./types").Effort }): Promise<void>;
  deleteChat(chatId: string): Promise<void>;
  createTask(t: { projectId: string; title: string; description: string; state?: TaskState; chatId?: string; effort?: import("./types").Effort }): Promise<Task>;
  updateTask(id: string, change: { title?: string; description?: string; state?: TaskState; effort?: import("./types").Effort }): Promise<Task>;
  deleteTask(id: string): Promise<void>;
  reorderTasks(projectId: string, ids: string[]): Promise<void>;
  makeProjectInternal(projectId: string): Promise<void>;
  /** Sends a message; the reply streams back through `onEvent`. */
  /** With `machineId`, the answer may use that computer's tools (each step needs approval). */
  send(chatId: string, text: string, machineId?: string, effort?: import("./types").Effort): Promise<void>;
  listActions(chatId: string): Promise<PcAction[]>;
  /** The lessons proposed in a chat (the project thread), oldest first. */
  listLessons(chatId: string): Promise<Lesson[]>;
  /** W2: run a task by itself on a computer, in a folder, checked by a command. */
  startTask(id: string, machineId: string, folder: string, check: string, testsMayChange?: boolean, effort?: import("./types").Effort): Promise<void>;
  /** Stops a running step: the computer kills the command and what it started. */
  stopAction(id: string): Promise<void>;
  /** Stops a running W2 task (and its running step). */
  stopTask(id: string): Promise<void>;
  /** A task's W2 runs, newest first (their ids open the report: /api/runs/<id>/report). */
  taskRuns(taskId: string): Promise<{ id: string; startedAt: string; endedAt: string | null; status: string; step: string | null; effort?: string | null; effortPicked?: boolean }[]>;
  /** Get cost lines for a task. */
  taskCosts(taskId: string): Promise<import("./types").CostLine | null>;
  /** Get weekly costs summary. */
  weeklyCosts(): Promise<import("./types").WeeklyCosts>;
  /** Does this folder exist on that computer, and which folders are in it (needs a read grant)? */
  checkFolder(machineId: string, path: string): Promise<{ state: "ok" | "nogrant" | "missing" | "notfolder" | "noanswer"; path: string; folders?: string[]; files?: number; message?: string }>;
  listActivity(): Promise<import("../views/activity").ActivityItem[]>;
  /** What Kompanion can use right now (W3): models, computers, tools, MCP servers, indexes, skills. */
  getCapabilities(): Promise<import("../views/capabilities").Capabilities>;
  /** GPU-01: Studio, Gaming or Auto for a computer with studio apps (admins). */
  setGpuMode(machine: string, mode: import("../views/capabilities").GpuModeName): Promise<void>;
  /** GPU-03: where the studio runs: "auto", a GPU id or "off" (admins). */
  setStudioTarget(target: string): Promise<void>;
  /** STU-01: the image types (workflows with a [studio] section), sorted by their order. */
  studioTypes(): Promise<import("../views/studio").StudioType[]>;
  /** STU-01: make 1 or 4 images; returns the run ids. */
  studioMake(type: string, prompt: string, size: string, count: 1 | 4, rating?: string, face?: { file: File; weight: number }): Promise<{ ids: string[] }>;
  /** STU-02: music or a sound effect; returns the run id. */
  studioAudio(kind: "music" | "sfx", prompt: string, lyrics: string, seconds: number): Promise<{ ids: string[] }>;
  /** STU-01: the signed-in user's runs, newest first. */
  studioMine(): Promise<import("../views/studio").StudioRun[]>;
  /** STU-02b: file n of a finished run becomes an asset of the project. */
  studioToAssets(runId: string, project: string, n?: number): Promise<{ assetId: number; project: string; name: string }>;
  /** M6-04: per GPU, samples, jobs and events of the last 1 or 24 hours. */
  gpuTimeline(hours: 1 | 24): Promise<import("../views/gputimeline").TlGpu[]>;
  /** One skill's SKILL.md, read-only. */
  getSkill(id: string, layer?: string): Promise<{ id: string; text: string; layer?: string }>;
  /** SK-03: a skill file's commits in its layer's repo, newest first. */
  skillHistory(layer: string, file: string): Promise<{ commits: import("../views/capabilities").SkillCommit[] }>;
  /** SK-03: saves a skill file as a commit in its layer's repo. */
  saveSkill(layer: string, file: string, text: string, message: string): Promise<{ commit: string }>;
  /** SK-03: moves one lesson line between the general and private layers (a commit in each repo). */
  moveLesson(from: string, to: string, file: string, line: string): Promise<{ to: string; from: string }>;
  /** W4: whether the server offers voice, and its voices. */
  voiceInfo(): Promise<{ enabled: boolean; voices: { id: string; label: string }[] }>;
  /** Speech to text: a recording from the microphone; lang "" lets Whisper detect it. */
  transcribe(audio: Blob, lang: string): Promise<string>;
  /** Text to speech: one or a few sentences as WAV audio. */
  speak(text: string, voice: string): Promise<Blob>;
  decideAction(id: string, decision: "approve" | "always" | "deny"): Promise<void>;
  /** Accept (with the text as edited) or dismiss a proposed lesson. */
  decideLesson(id: string, decision: "accept" | "dismiss", text?: string, layer?: "general" | "private"): Promise<void>;
  answer(taskId: string, optionId: string): Promise<void>;

  /** Live updates: streamed tokens, task progress, new messages. A "resync"
   * event means some updates were missed and lists should be reloaded. */
  onEvent(listener: (ev: ServerEvent) => void): () => void;
}

export type ServerEvent =
  | { type: "message"; message: Message }
  | { type: "message-delta"; messageId: string; chatId: string; text: string; done: boolean }
  | { type: "task"; task: Task }
  | { type: "machines"; machines: MachineStats[] }
  | { type: "changed"; what: "tasks" | "projects" | "chats" | "machines" | "access" | "settings" | "actions" | "lessons" | "project-assets" | "gpus" | "studio"; machineId?: string }
  | { type: "assets"; scan?: unknown; previews?: unknown } // Assets section news (api/assets.ts)
  | { type: "notify"; title: string; state: string; url: string } // for the phone app; the web app ignores it
  | { type: "resync" };
