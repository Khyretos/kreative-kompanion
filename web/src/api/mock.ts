// A fake server so the UI can be built and tried before the real one exists.
// Everything here is example data.
import { AUDIO_TYPES } from "./http";
import type { AccessEvent, GrantView } from "../views/access";
import type { KompanionApi, Lesson, ServerEvent } from "./client";
import type { PcAction } from "./client";
import { kindOf } from "../core/attachments";
import type { DevSettings, TaskState, AdminSettings, DaySummary, MachineStats, Chat, Fact, FactKind, FactsView, Message, ModelProvider, Project, RoleAssignment, SearchResult, Server, Task, ChatContext } from "./types";

const hoursFromNow = (h: number) => new Date(Date.now() + h * 3_600_000).toISOString();
const grants: Record<string, GrantView[]> = {
  // ACC-01: a permanent folder grant, a system grant ending soon (root) and one that ended yesterday.
  soucouyant: [
    { target: "/home/kees/projects/kompanion", rights: ["read", "write"], grantedBy: "demo", grantedAt: "2026-10-01T10:00:00Z", expires: null },
    { target: "system", rights: ["packages", "root"], grantedBy: "demo", grantedAt: hoursFromNow(-21), expires: hoursFromNow(3) },
    { target: "/home/kees/old-notes", rights: ["read"], grantedBy: "demo", grantedAt: hoursFromNow(-48), expires: hoursFromNow(-24) },
  ],
};
const actions: PcAction[] = [];
const projectAssets: Record<string, import("../views/projectpanel").ProjectAsset[]> = {};
const emptyDev = (): DevSettings => ({ preset: "", engine: "", build: "", test: "", run: "", deploy: "", screenshots: "", last: {} });
const DEV_PRESETS: DevSettings[] = [
  { ...emptyDev(), preset: "software", build: "make", test: "make test" },
  { ...emptyDev(), preset: "game", engine: "kk-engine", build: "cmake -B build -G Ninja -DCMAKE_BUILD_TYPE=Debug && cmake --build build -j --target kke_tests starter_game", test: "./build/bin/kke_tests", run: "cd build/bin && xvfb-run -a -s '-screen 0 1280x720x24' timeout 8 ./starter_game; test $? -eq 124", screenshots: "screenshots" },
];
const devSettings: Record<string, DevSettings> = {};
// PROJ-OVERVIEW: each project's About text and links (the demo keeps them beside the project list).
const overviewText: Record<string, { about: string; links: import("./types").ProjectLink[] }> = {
  "p-kk": {
    about: "A small C++17 engine with a Vulkan renderer, made for Kees's own games.\n\n" +
      "It keeps the editor, the asset pipeline and the runtime in one repo, so a game can be built and tested from Kompanion. " +
      "The renderer targets desktop Linux and Windows first; consoles are out of scope.\n\n" +
      "## Goals\n\n- Fast iteration: a scene reloads in under a second\n- Tests for every system\n- Screenshots for every visual change",
    links: [{ label: "Repo", url: "https://git.kreative-kompas.com/khyretos/kk-engine" }],
  },
};
const history: AccessEvent[] = [];

const now = Date.now();
const ago = (min: number) => new Date(now - min * 60_000).toISOString();
const later = (min: number) => new Date(now + min * 60_000).toISOString();
let seq = 100;
/** GPU-01: soucouyant's GPU mode in the demo. */
let soucouyantMode: import("../views/capabilities").GpuModeName = "auto";
let soucouyantStopped = false;
/** GPU-03: "Studio runs on" in the demo. */
let studioTarget = "auto";
/** CHAT-03b: knowledge collections of the demo user. */
const knowledgeCols: import("../views/knowledge").KCollection[] = [
  { id: "kc-godot", name: "Godot Assistant", source: "openwebui", docs: 412, chunks: 1830, vectors: 1208, projects: [],
    documents: [{ id: 1, name: "classes/class_timer.md", chars: 5210, addedAt: ago(60 * 30) }, { id: 2, name: "tutorials/signals.md", chars: 9120, addedAt: ago(60 * 30) }] },
  { id: "kc-notes", name: "Game notes", source: "app", docs: 2, chunks: 5, vectors: 5, projects: ["p-kk"],
    documents: [{ id: 3, name: "combat-design.md", chars: 3400, addedAt: ago(90) }, { id: 4, name: "level-ideas.pdf", chars: 2100, addedAt: ago(30) }] },
];
/** STU-01: the demo's Studio library (newest first). */
const demoPic = (hue: number) => "data:image/svg+xml," + encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="96" height="96"><rect width="96" height="96" fill="hsl(${hue} 60% 45%)"/><circle cx="48" cy="40" r="18" fill="#f4eefc"/></svg>`);
// 3D-01: an 848-byte purple cube as a binary glTF, for the demo "3D model" run.
const demoGlb = "data:model/gltf-binary;base64,Z2xURgIAAABQAwAAjAIAAEpTT057ImFzc2V0Ijp7InZlcnNpb24iOiIyLjAifSwic2NlbmUiOjAsInNjZW5lcyI6W3sibm9kZXMiOlswXX1dLCJub2RlcyI6W3sibWVzaCI6MCwibmFtZSI6ImRlbW8ifV0sIm1lc2hlcyI6W3sicHJpbWl0aXZlcyI6W3siYXR0cmlidXRlcyI6eyJQT1NJVElPTiI6MH0sImluZGljZXMiOjEsIm1hdGVyaWFsIjowfV19XSwibWF0ZXJpYWxzIjpbeyJwYnJNZXRhbGxpY1JvdWdobmVzcyI6eyJiYXNlQ29sb3JGYWN0b3IiOlswLjQyLDAuMjUsMC44MywxXSwibWV0YWxsaWNGYWN0b3IiOjAsInJvdWdobmVzc0ZhY3RvciI6MC44fX1dLCJidWZmZXJzIjpbeyJieXRlTGVuZ3RoIjoxNjh9XSwiYnVmZmVyVmlld3MiOlt7ImJ1ZmZlciI6MCwiYnl0ZU9mZnNldCI6MCwiYnl0ZUxlbmd0aCI6OTYsInRhcmdldCI6MzQ5NjJ9LHsiYnVmZmVyIjowLCJieXRlT2Zmc2V0Ijo5NiwiYnl0ZUxlbmd0aCI6NzIsInRhcmdldCI6MzQ5NjN9XSwiYWNjZXNzb3JzIjpbeyJidWZmZXJWaWV3IjowLCJjb21wb25lbnRUeXBlIjo1MTI2LCJjb3VudCI6OCwidHlwZSI6IlZFQzMiLCJtaW4iOlstMC41LC0wLjUsLTAuNV0sIm1heCI6WzAuNSwwLjUsMC41XX0seyJidWZmZXJWaWV3IjoxLCJjb21wb25lbnRUeXBlIjo1MTIzLCJjb3VudCI6MzYsInR5cGUiOiJTQ0FMQVIifV19ICAgqAAAAEJJTgAAAAC/AAAAvwAAAL8AAAA/AAAAvwAAAL8AAAA/AAAAPwAAAL8AAAC/AAAAPwAAAL8AAAC/AAAAvwAAAD8AAAA/AAAAvwAAAD8AAAA/AAAAPwAAAD8AAAC/AAAAPwAAAD8AAAIAAQAAAAMAAgAEAAUABgAEAAYABwAAAAEABQAAAAUABAACAAMABwACAAcABgABAAIABgABAAYABQAAAAQABwAAAAcAAwA=";
const studioRuns: import("../views/studio").StudioRun[] = [
  { id: "sr5", type: "image-to-3d", gpu: "rx9070", prompt: "3d model", size: "square", state: "done", error: null, files: [demoGlb, demoPic(150)], startedAt: ago(20), endedAt: ago(18) },
  { id: "sr1", type: "character", gpu: "rx9070", prompt: "a cheerful fox adventurer with a green scarf", size: "tall", state: "done", error: null, files: [demoPic(28)], startedAt: ago(30), endedAt: ago(29) },
  { id: "sr2", type: "character", gpu: "a770", prompt: "a knight in silver armour", size: "square", state: "failed", error: "ComfyUI stopped answering", files: [], startedAt: ago(40), endedAt: ago(39) },
  { id: "sr3", type: "oc-sheet", gpu: "rx9070", prompt: "an adult woman knight with silver hair", size: "wide", state: "done", error: null, check: { ok: true, retries: 2, issues: [] }, files: [demoPic(270)], startedAt: ago(50), endedAt: ago(45) },
  { id: "sr4", type: "oc-sheet", gpu: "rx9070", prompt: "a man in a red hoodie", size: "wide", state: "done", error: null, check: { ok: false, retries: 3, issues: ["back: face seen (head facing the wrong way)"] }, files: [demoPic(200)], startedAt: ago(60), endedAt: ago(52) },
];
const id = (p: string) => `${p}${++seq}`;

const projects: Project[] = [
  { id: "p-kompanion", name: "Kreative Kompanion", description: "This app", updatedAt: ago(2) },
  { id: "p-kk", name: "kk-engine", description: "C++17 Vulkan game engine", updatedAt: ago(14) },
  { id: "p-3dco", name: "3dco-plus", description: "3D controller overlay for streamers", updatedAt: ago(60 * 26) },
  { id: "p-nohboard", name: "nohboard-qt", description: "Keyboard overlay, Qt", updatedAt: ago(60 * 24 * 4) },
];

const lessons: Lesson[] = [];
const chats: Chat[] = [
  { id: "c-kk", title: "Debug tooling and tests", projectId: "p-kk", updatedAt: ago(14) },
  { id: "c-kk-2", title: "Renderer performance", projectId: "p-kk", updatedAt: ago(60 * 5) },
  { id: "c-kk-thread", title: "Project thread", projectId: "p-kk", updatedAt: ago(30), thread: true },
  { id: "c-kompanion", title: "First UI draft", projectId: "p-kompanion", updatedAt: ago(2) },
  { id: "c-3dco", title: "SDL3 port", projectId: "p-3dco", updatedAt: ago(60 * 26) },
  { id: "c-nohboard", title: "Wayland input capture", projectId: "p-nohboard", updatedAt: ago(60 * 24 * 4) },
  { id: "c-blender", title: "Blender scene", updatedAt: ago(60 * 24 * 5) },
  { id: "c-sources", title: "Sourced answer", updatedAt: ago(60 * 24 * 6) },
  { id: "c-loose", title: "Which local model next?", updatedAt: ago(60 * 3) },
];

// FACTS: kk-engine's facts and the "Debug tooling and tests" chat's own; one path fact is flagged.
const fact = (id: string, scope: { projectId?: string; chatId?: string }, key: string, value: string, kind: FactKind, source: "user" | "assistant", min: number, flagged = false): Fact =>
  ({ id, projectId: scope.projectId ?? null, chatId: scope.chatId ?? null, key, value, kind, source, createdAt: ago(min + 60), updatedAt: ago(min), lastUsedAt: ago(5), flaggedAt: flagged ? ago(30) : null });
const facts: Fact[] = [
  fact("f1", { projectId: "p-kk" }, "Build command", "cmake --build build -j8", "other", "user", 60 * 24 * 6),
  fact("f2", { projectId: "p-kk" }, "Engine repo", "/home/kees/projects/kk-engine", "path", "user", 60 * 24 * 9),
  fact("f3", { projectId: "p-kk" }, "Validation default", "--vk-validation is off in release builds, on in debug.", "text", "assistant", 50),
  fact("f4", { projectId: "p-kk" }, "Docs", "https://kk-engine.kreative-kompas.com/docs", "url", "assistant", 60 * 24 * 2),
  fact("f5", { projectId: "p-kk" }, "Shader cache", "/home/kees/.cache/kk-engine/shaders", "path", "assistant", 60 * 24 * 3, true),
  fact("f6", { chatId: "c-kk" }, "Benchmark slot", "Tonight 01:00, when the A770 is free", "text", "assistant", 40),
  fact("f7", { chatId: "c-kk" }, "Test runner", "ctest --output-on-failure", "other", "user", 90),
];
const sortFacts = (list: Fact[]) => [...list].sort((a, b) => Number(!!b.flaggedAt) - Number(!!a.flaggedAt) || a.key.toLowerCase().localeCompare(b.key.toLowerCase()));
// CTX-01: the context meter's demo numbers: one chat close to full with a summary, one with room.
const contexts: Record<string, ChatContext> = {
  "c-kk": { chatId: "c-kk", used: 104_300, window: 128_000, model: "deepseek-chat", summary: {
    messages: 14, at: ago(40),
    text: "Kees wants a --vk-validation flag (off by default in release builds) and an engine.toml setting.\n- ModuleLoader tests: cover the missing-file and the circular-import cases.\n- README build steps updated (done).\n- Open: benchmark at 1440p needs the GPU; asked when it can run.",
  } },
  "c-kk-2": { chatId: "c-kk-2", used: 18_400, window: 128_000, model: "deepseek-chat", summary: null },
  "c-3dco": { chatId: "c-3dco", used: 123_500, window: 128_000, model: "deepseek-chat", summary: null },
};

const adminSettings: AdminSettings = {
  appName: "Kreative Kompanion", smtpHost: "", smtpPort: 587, smtpTls: "starttls", smtpUser: "", smtpFrom: "", smtpReplyTo: "",
  colorBrand: "#5c398e", colorLinkDark: "#f3941f", colorLinkLight: "#8f4700", colorAccent: "#f3941f",
};

const messages: Message[] = [
  // BLD-01: a Blender render from a chat tool, shown as a picture in the answer.
  { id: "m-bl1", chatId: "c-blender", author: "user", at: ago(60 * 24 * 5), text: "Render a monkey on a plane." },
  {
    id: "m-bl2", chatId: "c-blender", author: "orchestrator", at: ago(60 * 24 * 5 - 1),
    text: "*Looked up Blender: blender_render (bpy.ops.mesh.primitive_monkey_add())*\n\n![Blender: blender_render](/api/chats/c-blender/files/0f8e2c1a-5b7d-4c3e-9a21-6d4f0b9e7c55.png)\n\n[Download scene.blend](/api/chats/c-blender/files/1a2b3c4d-5b7d-4c3e-9a21-6d4f0b9e7c66.blend)\n\n" +
      "Here is Suzanne on a grey plane, lit by a sun from the upper left.\n\n```python\nbpy.ops.mesh.primitive_monkey_add()\n```\n\n![tracker](https://example.com/pixel.png)",
  },
  // CHAT-04: an answer with inline source names and the list of sources under it.
  { id: "m-so1", chatId: "c-sources", author: "user", at: ago(60 * 24 * 6), text: "What does the Timer node do, and when is Godot 4.5 out?" },
  {
    id: "m-so2", chatId: "c-sources", author: "orchestrator", at: ago(60 * 24 * 6 - 1),
    text: "*Looked up Web: web_search (godot 4.5 release)*\n\nThe Timer node counts down an interval and emits `timeout` ([Godot Assistant / timer.md](src:2)). Godot 4.5 was released in September ([Godot Engine](https://godotengine.org/releases/4.5/)).\n\n" +
      ":::sources\n" + JSON.stringify([
        { n: 1, name: "Godot Engine", url: "https://godotengine.org/releases/4.5/", excerpt: "Godot 4.5 is out: shader baking, accessibility support, and more." },
        { n: 2, name: "Godot Assistant / timer.md", url: null, excerpt: "The Timer node counts down an interval and emits timeout when it reaches 0." },
      ]) + "\n:::",
  },
  {
    id: "m1", chatId: "c-kk", author: "user", at: ago(20),
    text: "I keep turning the Vulkan validation layers on and off by editing code. Can you make that a setting, and finally add tests for the module loader?",
  },
  {
    id: "m2", chatId: "c-kk", author: "orchestrator", at: ago(19),
    text:
      "I split this into four tasks. Qwen does the work in a container on **soucouyant**, DeepSeek reviews each result before anything touches your repo.\n\n" +
      "- Add a `--vk-validation` flag and an `engine.toml` setting\n- Write unit tests for `ModuleLoader`\n- Update the README build steps\n- Benchmark the renderer at 1440p, to check the flag costs nothing when off\n\n" +
      "The benchmark needs the GPU, so I asked you when it can run.",
    taskIds: ["t1", "t2", "t3", "t4"],
  },
  {
    id: "m3", chatId: "c-kk", author: "user", at: ago(15),
    text: "Good. And what about the SDL3 input port in 3dco-plus, can that run too?",
  },
  {
    id: "m4", chatId: "c-kk", author: "orchestrator", at: ago(14),
    text: "That one is in the 3dco-plus project. It is waiting on one choice from you: keep the raw joystick fallback, or rely on SDL3's gamepad mapping only. The card is in the task list.",
  },
  {
    id: "m5", chatId: "c-kompanion", author: "user", at: ago(3),
    text: "Start building the first draft, starting with the UI.",
  },
  {
    id: "m6", chatId: "c-kompanion", author: "orchestrator", at: ago(2),
    text: "This is that draft. Projects and chats on the left, me in the middle, running tasks on the right. Everything you see is example data from a fake server.",
  },
  {
    id: "m7", chatId: "c-loose", author: "user", at: ago(60 * 3),
    text: "Is there a local model that fits on the A770 and beats Qwen3.5-9B at tool calls?",
  },
  {
    id: "m8", chatId: "c-loose", author: "orchestrator", at: ago(60 * 3 - 1),
    text: "I can test that for you. Each candidate runs the worker test set (42 tasks from past failures) at night, and you get a pass-rate comparison in the morning.",
  },
];

const tasks: Task[] = [
  {
    id: "t1", projectId: "p-kk", title: "Add a --vk-validation flag and setting",
    state: "running", progress: 0.62, step: "Building in the container", role: "worker",
    // COMPACT-UI: a run is going and did something seconds ago (the panel shows it live).
    activeRun: true, lastActivity: new Date(Date.now() - 5_000).toISOString(),
    model: "Qwen3.5-9B", runner: "soucouyant (Linux)", workspace: "Container · 4 cores · 8 GB",
    events: [
      { kind: "step", at: ago(18), text: "Read the code map: Instance.cpp, Config.cpp, main.cpp" },
      { kind: "tool", at: ago(17), tool: "files.write", detail: "src/core/Config.cpp", ok: true },
      { kind: "diff", at: ago(17), file: "src/core/Config.cpp", added: 14, removed: 2 },
      { kind: "diff", at: ago(16), file: "src/render/Instance.cpp", added: 9, removed: 5 },
      { kind: "tool", at: ago(15), tool: "shell", detail: "cmake --build build -j4", ok: false },
      {
        kind: "call", at: ago(15), call: {
          id: "call-1", role: "worker", model: "Qwen3.5-9B", provider: "OVMS on kireserver",
          reason: "The build failed, so the worker gets the compiler error and the two files it touched.",
          request: {
            system: "You are the worker. Change only the files listed. Answer with a JSON patch.",
            context: ["skill: worker/cpp-includes", "skill: worker/cmake", "file: src/render/Instance.cpp (lines 1-60)", "build log: 12 lines"],
            prompt: "Instance.cpp:41: error: 'string_view' is not a member of 'std'. Fix the build error.",
          },
          response: '{"file":"src/render/Instance.cpp","insert_after_line":3,"text":"#include <string_view>"}',
          tokensIn: 2140, tokensOut: 38, ms: 1900, costEur: 0, energyWh: 0.09,
        },
      },
      { kind: "step", at: ago(15), text: "Fixed missing include <string_view>, rebuilding" },
    ],
  },
  {
    id: "t2", projectId: "p-kk", title: "Write unit tests for ModuleLoader",
    state: "in_review", progress: 0.9, step: "DeepSeek is reviewing", role: "reviewer",
    model: "DeepSeek", runner: "soucouyant (Linux)", workspace: "Container · 2 cores · 4 GB",
    events: [
      { kind: "diff", at: ago(12), file: "tests/ModuleLoaderTests.cpp", added: 118, removed: 0 },
      { kind: "tool", at: ago(10), tool: "shell", detail: "ctest --output-on-failure  (11/11 passed)", ok: true },
      { kind: "step", at: ago(9), text: "Cheap checks passed, sent to the reviewer" },
    ],
  },
  {
    id: "t3", projectId: "p-kk", title: "Update the README build steps",
    state: "done", progress: 1, step: "Merged into branch kompanion/vk-validation", role: "worker",
    model: "Qwen3.5-9B", runner: "soucouyant (Linux)",
    events: [
      { kind: "diff", at: ago(13), file: "README.md", added: 6, removed: 3 },
      {
        kind: "call", at: ago(12), call: {
          id: "call-2", role: "reviewer", model: "deepseek-chat", provider: "DeepSeek",
          reason: "Cheap checks passed (Markdown lint), so the README change goes to the reviewer.",
          request: {
            system: "You are the reviewer. Check the change against the repository. Reply pass or fail with one reason.",
            context: ["skill: reviewer/rubric", "diff: README.md (+6 -3)", "file: CMakeLists.txt (options only)"],
            prompt: "Review this README change.",
          },
          response: "fail: README mentions -DKK_ENABLE_VALIDATION=ON, but CMakeLists.txt defines no such option.",
          tokensIn: 3310, tokensOut: 41, ms: 2600, costEur: 0.001,
        },
      },
      { kind: "review", at: ago(12), model: "DeepSeek", verdict: "fail", note: "Invented a CMake option KK_ENABLE_VALIDATION that does not exist." },
      { kind: "lesson", at: ago(12), skill: "worker/cmake", note: "Only document options found in CMakeLists.txt; list them with cmake -LH first." },
      { kind: "diff", at: ago(11), file: "README.md", added: 5, removed: 3 },
      { kind: "review", at: ago(10), model: "DeepSeek", verdict: "pass", note: "Matches CMakeLists.txt." },
    ],
  },
  ...["Profile shader compile times", "Split the asset loader into its own module", "Document the console commands"].map((title, i) => ({
    id: `t-kk-${i + 1}`, projectId: "p-kk", title, description: `**Goal:** ${title}.`,
    state: "queued" as const, progress: 0, step: "Waiting", role: "worker" as const, model: "Qwen3.5-9B", events: [],
  })),
  {
    id: "t4", projectId: "p-kk", title: "Benchmark the renderer at 1440p",
    state: "waiting_resources", progress: 0, step: "Needs the GPU", role: "worker",
    model: "Qwen3.5-9B", runner: "soucouyant (Linux)", workspace: "Container · 8 cores · 16 GB · GPU",
    events: [{ kind: "step", at: ago(19), text: "Steam is running, so I asked before taking the GPU" }],
    question: {
      kind: "resources",
      text: "This needs 8 cores, 16 GB and the GPU on soucouyant for about 40 minutes.",
      options: [
        { id: "now", label: "Now" },
        { id: "tonight", label: "Tonight", detail: "01:00", recommended: true },
        { id: "less", label: "With less", detail: "4 cores, no GPU" },
      ],
    },
  },
  {
    id: "t5", projectId: "p-3dco", title: "Port the input layer to SDL3",
    state: "needs_input", progress: 0.3, step: "Waiting for your choice", role: "orchestrator",
    model: "DeepSeek", runner: "soucouyant (Linux)",
    events: [
      { kind: "step", at: ago(60 * 26), text: "Mapped every SDL2 input call (37 call sites)" },
      { kind: "screenshot", at: ago(60 * 26), caption: "Overlay running on SDL3 in the headless workspace" },
    ],
    question: {
      kind: "choice",
      text: "Keep the raw joystick fallback, or use only SDL3's gamepad mapping?",
      options: [
        { id: "keep", label: "Keep fallback", detail: "Odd controllers keep working", recommended: true },
        { id: "drop", label: "SDL3 only", detail: "Less code to maintain" },
      ],
    },
  },
  {
    id: "t7", projectId: "p-nohboard", title: "Install Qt 6.8 build dependencies",
    state: "needs_input", progress: 0.1, step: "Waiting for your approval", role: "worker",
    model: "Qwen3.5-9B", runner: "soucouyant (Linux)",
    events: [{ kind: "step", at: ago(40), text: "CMake needs Qt6::WaylandClient, which is not installed" }],
    question: {
      kind: "approval",
      text: "Run this command on your own system (not in a container)?",
      action: {
        machine: "soucouyant (Linux)", cwd: "~/development/nohboard-qt",
        command: "sudo pacman -S --needed qt6-wayland qt6-tools",
        network: ["mirror.cachyos.org"],
      },
      options: [
        { id: "once", label: "Allow once", recommended: true },
        { id: "always", label: "Always allow", detail: "pacman -S qt6-*" },
        { id: "reject", label: "Deny" },
      ],
    },
  },
  {
    id: "t6", projectId: "p-kompanion", title: "Scaffold the Rust server",
    state: "queued", progress: 0, step: "Starts after the UI draft", role: "worker",
    model: "Qwen3.5-9B", runner: "kireserver", scheduledFor: later(60), events: [],
  },
];
// COMPACT-UI: with __kkDemoBoard set (tests, screenshots), a task "in progress" on the Windshift
// board that Kompanion never ran, as on kireserver; the panel says so instead of pulsing.
if ((globalThis as { __kkDemoBoard?: boolean }).__kkDemoBoard) {
  tasks.push({ id: "t-board", projectId: "p-kk", title: "Backups and monitoring", state: "running", progress: 0, step: "", role: "worker", model: "",
    updatedAt: new Date(Date.now() - 21 * 86_400_000).toISOString(), activeRun: false, lastActivity: null, events: [] });
}

const providers: ModelProvider[] = [
  {
    id: "ovms", name: "OVMS on kireserver", kind: "openai-compatible", baseUrl: "https://ai.example.lan/v3", local: true,
    models: [{ id: "qwen3.5-9b", contextTokens: 32768, tokensPerSecond: 38, toolCalls: true, jsonSchema: true }],
  },
  {
    id: "deepseek", name: "DeepSeek", kind: "openai-compatible", baseUrl: "https://api.deepseek.com", local: false,
    models: [{ id: "deepseek-chat", contextTokens: 131072, toolCalls: true, jsonSchema: true }],
  },
  {
    id: "anthropic", name: "Claude (Anthropic API)", kind: "anthropic", baseUrl: "https://api.anthropic.com", local: false,
    models: [{ id: "claude-sonnet-5-5", contextTokens: 200000, toolCalls: true, jsonSchema: true }],
  },
];

const overseerPrefs = { name: "Loquendo", interject: true };
let demoMaxSteps = 10; // AUTO-01
let demoExplain: import("./types").ExplainPrefs = { on: false, verbose: false }; // CMD-01
const demoNotify: import("./types").NotificationPrefs = { email: "kees@example.com", onNeedsInput: true, onFailed: true, onDone: false, dailySummary: false,
  onApproval: true, mailOn: true, pushOn: true, quietFrom: "", quietTo: "", tzOffset: 0 }; // NOTIFY
const demoCustom: import("./types").CustomProvider[] = []; // PROV-UI

let roles: RoleAssignment[] = [
  { role: "orchestrator", providerId: "deepseek", modelId: "deepseek-chat" },
  { role: "worker", providerId: "ovms", modelId: "qwen3.5-9b" },
  { role: "reviewer", providerId: "deepseek", modelId: "deepseek-chat" },
  { role: "overseer", providerId: "ovms", modelId: "qwen3.5-9b" },
];

const machines: MachineStats[] = [
  {
    id: "kireserver", name: "kireserver", isServer: !(globalThis as { __kkDemoNoHost?: boolean }).__kkDemoNoHost, os: "Ubuntu", online: true, cpu: 0.34, ramUsedGb: 41, ramTotalGb: 64,
    kompanionShare: 0.12, history: [92, 95, 140, 151, 148, 150, 97, 94, 149, 152, 150, 149],
    gpus: [{ name: "Arc A770", load: 0.71, vramUsedGb: 11.2, vramTotalGb: 16, watts: 162, tempC: 67, use: "Qwen3.5-9B (OVMS)" }],
  },
  {
    id: "soucouyant", name: "soucouyant", os: "CachyOS", online: true, runnerVersion: "0.5.0", runnerLatest: "0.6.0", cpu: 0.58, ramUsedGb: 19, ramTotalGb: 32,
    kompanionShare: 0.31, busy: "Steam is running", history: [180, 190, 260, 255, 270, 262, 258, 265, 270, 268, 266, 271],
    gpus: [{ name: "GPU", load: 0.64, vramUsedGb: 6.1, vramTotalGb: 8, watts: 141, tempC: 72, use: "Game (not Kompanion)" }],
  },
  {
    id: "macbook", name: "MacBook", os: "macOS", online: false, cpu: 0, ramUsedGb: 0, ramTotalGb: 16,
    kompanionShare: 0, history: [], gpus: [],
  },
  {
    id: "desktop-387pphp", name: "DESKTOP-387PPHP", os: "Windows", online: false, cpu: 0, ramUsedGb: 0, ramTotalGb: 32,
    kompanionShare: 0, history: [], gpus: [],
  },
];

const servers: Server[] = [
  { url: "https://kompanion.home.arpa", name: "kireserver", version: "0.1.0", discovered: true },
];

const replies = [
  "Here is how I would split that up. I will check each piece with the cheap checks first, then the reviewer.\n\n- Read the relevant files through the code map\n- Make the change in a container workspace\n- Run the build and tests\n\nI added it to the task list on the right.",
  "Got it. I made that a task for Qwen with a small, self-contained scope. If it fails review I will write the lesson into the matching skill so the next attempt starts smarter.",
];

// ARTIFACTS: a reply with an HTML page and a long code block, for the side pane (prompt: "artifact demo").
const ARTIFACT_HTML = `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Medabots</title>
  <style>
    body { font-family: system-ui, sans-serif; margin: 0; background: #1c1329; color: #f4eefc; }
    main { max-width: 32rem; margin: 3rem auto; padding: 0 1rem; }
    h1 { color: #cca9ff; }
    button { font: inherit; padding: 0.5rem 1rem; border-radius: 6px; border: 0; background: #f3941f; color: #0c0917; }
  </style>
</head>
<body>
  <main>
    <h1>Medabots</h1>
    <p>Two bots, four parts each. The weakest part takes the hit.</p>
    <button id="fight">Fight a round</button>
    <p id="log">No rounds yet.</p>
  </main>
  <script>
    let round = 0;
    document.getElementById("fight").onclick = () => {
      round += 1;
      document.getElementById("log").textContent = "Round " + round + ": the left arm takes 3 damage.";
    };
  </script>
</body>
</html>`;
const ARTIFACT_TS = `export interface Part { name: string; armor: number; attack: number }\n\nexport interface Bot {\n  name: string;\n  head: Part;\n  leftArm: Part;\n  rightArm: Part;\n  legs: Part;\n}\n\n/** Damage one attack does: attack minus half the armor, never below 1. */\nexport function damage(from: Part, to: Part): number {\n  return Math.max(1, from.attack - Math.floor(to.armor / 2));\n}\n\n/** The part that takes the hit: the weakest one that still works. */\nexport function target(bot: Bot, broken: Set<keyof Bot>): keyof Bot {\n  const parts = ["head", "leftArm", "rightArm", "legs"] as const;\n  const alive = parts.filter((p) => !broken.has(p));\n  return alive.sort((a, b) => bot[a].armor - bot[b].armor)[0] ?? "head";\n}\n\nexport function turn(a: Bot, b: Bot, broken: Set<keyof Bot>): number {\n  const hit = target(b, broken);\n  return damage(a.rightArm, b[hit] as Part);\n}`;
const ARTIFACT_REPLY = "Here is a first page for the battle rules, and the damage code it will use.\n\n```html\n" + ARTIFACT_HTML + "\n```\n\n```ts\n" + ARTIFACT_TS + "\n```\n\nOpen either one to see it whole; the page runs in a sandbox with no network.";

// PROJECT-SHARING: the members each project's Overview lists (the owner's own projects start with none), and the demo users the add form accepts. ?demo&shared also adds a project mara shared with you as a viewer, with one chat.
const members: Record<string, import("./types").ProjectMember[]> = {};
const DEMO_USERS: Record<string, string> = { mara: "u-mara", kees: "u-me" };

if (typeof location !== "undefined" && new URLSearchParams(location.search).has("shared")) {
  projects.push({ id: "p-shared", name: "Mara's shop", description: "Plants and pots", updatedAt: ago(40), role: "viewer", ownerName: "mara" });
  chats.push({ id: "c-shared", title: "Shop plan", projectId: "p-shared", updatedAt: ago(40) });
  messages.push({ id: "m-shared-1", chatId: "c-shared", author: "user", at: ago(41), text: "Which plants sell best?" });
  members["p-shared"] = [{ userId: "u-me", name: "kees", role: "viewer", you: true }];
}

export class MockApi implements KompanionApi {
  private listeners = new Set<(ev: ServerEvent) => void>();
  private replyIndex = 0;

  constructor() {
    setInterval(() => this.tick(), 1200);
  }

  private emit(ev: ServerEvent) {
    this.listeners.forEach((l) => l(ev));
  }

  private tick() {
    for (const m of machines) {
      if (!m.online) continue;
      const jitter = () => (Math.random() - 0.5) * 0.08;
      m.cpu = clamp(m.cpu + jitter());
      for (const g of m.gpus) {
        g.load = clamp((g.load ?? 0) + jitter());
        g.watts = Math.round(40 + g.load * 175);
      }
      m.history = [...m.history.slice(-23), Math.round(m.gpus.reduce((w, g) => w + (g.watts ?? 0), 0) + 60 * m.cpu)];
    }
    this.emit({ type: "machines", machines: structuredClone(machines) });
    for (const t of tasks) {
      if (t.state !== "running") continue;
      t.progress = Math.min(1, t.progress + 0.03);
      if (t.progress >= 1) {
        t.state = "in_review";
        t.step = "DeepSeek is reviewing";
        t.events.push({ kind: "step", at: new Date().toISOString(), text: "Cheap checks passed, sent to the reviewer" });
      } else if (t.progress > 0.8 && t.step !== "Running the tests") {
        t.step = "Running the tests";
        t.events.push({ kind: "tool", at: new Date().toISOString(), tool: "shell", detail: "ctest --output-on-failure", ok: true });
      }
      this.emit({ type: "task", task: structuredClone(t) });
    }
  }

  async discover() {
    await wait(500);
    return servers;
  }
  async connect(url: string) {
    await wait(400);
    const clean = url.trim().replace(/\/+$/, "");
    if (!/^https?:\/\/[^\s]+$/.test(clean)) throw new Error("That doesn't look like a link. It should start with https://");
    return { url: clean, name: new URL(clean).hostname, version: "0.1.0" };
  }
  async status() {
    const off = (globalThis as { __kkDemoFeatures?: Partial<import("./types").Features> }).__kkDemoFeatures ?? {};
    return { name: "Kreative Kompanion (demo)", machineName: "kireserver", mcp: ["Web", "Stack Overflow", "Developer docs", "History", "Memory"], overseer: { ...overseerPrefs }, maxSteps: demoMaxSteps, explain: { ...demoExplain }, mcpInfo: { "Stack Overflow": "3.5 million answered programming questions", "Developer docs": "Indexed developer documentation" }, version: "0.1.0", setupNeeded: false, user: "Kees", admin: true, adult: !!(globalThis as { __kkDemoAdult?: boolean }).__kkDemoAdult, theme: "system" as const,
      features: { assets: true, gpus: true, voice: true, windshift: true, ...off } };
  }
  async setup() {}
  async logout() { return null; }
  async setTheme() {}
  async setMachinesRefresh() {}
  async setGpuPins() {}
  async setMaxSteps(n: number) { demoMaxSteps = n; }
  async setExplainPrefs(p: import("./types").ExplainPrefs) { demoExplain = { ...p }; }
  // PROV-UI: providers added in Settings (checked like the server) and new projects.
  async listCustomProviders() { return structuredClone({ providers: demoCustom, secretDirs: ["/run/secrets", "/data/secrets"] }); }
  private checkProvider(p: import("./types").ProviderInput) {
    if (!p.name.trim()) throw new Error("Give the provider a name of up to 60 characters.");
    if (!/^https?:\/\/[^/\s]+/.test(p.baseUrl.trim())) throw new Error("The address starts with https:// (or http:// on your own network).");
    if (p.apiKeyEnv && !/^[A-Z][A-Z0-9_]*_(KEY|TOKEN)$/.test(p.apiKeyEnv)) throw new Error("The key's environment variable is in capitals and ends in _KEY or _TOKEN, like XAI_API_KEY.");
    if (p.apiKeyFile && !/^\/(run\/secrets|data\/secrets)\/[^.][^/]*$/.test(p.apiKeyFile)) throw new Error("The key file must be in /run/secrets or /data/secrets.");
  }
  private showProvider(c: import("./types").CustomProvider) {
    const models = /x\.ai/.test(c.baseUrl) ? [{ id: "grok-4" }, { id: "grok-4-fast" }] : [{ id: "default" }];
    const shown: ModelProvider = { id: c.id, name: c.name, kind: c.kind, baseUrl: c.baseUrl, local: c.local, models };
    const i = providers.findIndex((p) => p.id === c.id);
    if (i >= 0) providers[i] = shown; else providers.push(shown);
  }
  async addProvider(p: import("./types").ProviderInput) {
    this.checkProvider(p);
    let id = p.name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "provider";
    for (let n = 2; providers.some((x) => x.id === id); n++) id = `${id.replace(/-\d+$/, "")}-${n}`;
    const c = { id, ...p, kind: p.kind as "openai-compatible" | "anthropic", baseUrl: p.baseUrl.trim().replace(/\/$/, ""), hasKey: !!p.apiKeyEnv };
    demoCustom.push(c);
    this.showProvider(c);
    return structuredClone(c);
  }
  async updateProvider(pid: string, p: import("./types").ProviderInput) {
    this.checkProvider(p);
    const c = demoCustom.find((x) => x.id === pid);
    if (!c) throw new Error("That provider is gone.");
    Object.assign(c, p, { baseUrl: p.baseUrl.trim().replace(/\/$/, ""), hasKey: !!p.apiKeyEnv });
    this.showProvider(c);
    return structuredClone(c);
  }
  async deleteProvider(pid: string) {
    const used = roles.filter((r) => r.providerId === pid).map((r) => r.role);
    if (used.length) throw new Error(`Roles still use it (${used.join(", ")}): pick another model for them first.`);
    demoCustom.splice(demoCustom.findIndex((x) => x.id === pid), 1);
    providers.splice(providers.findIndex((x) => x.id === pid), 1);
  }
  async createProject(p: { name: string; description?: string; ptype?: import("./types").ProjectType }) {
    const name = p.name.trim().replace(/\s+/g, " ");
    if (!name) throw new Error("Name the project (up to 80 characters).");
    if (projects.some((x) => !x.archivedAt && x.name.toLowerCase() === name.toLowerCase())) throw new Error(`You already have a project called "${name}".`);
    const proj: Project = { id: id("p"), name, description: p.description ?? "", updatedAt: new Date().toISOString(), type: p.ptype ?? "chat" };
    projects.unshift(proj);
    this.emit({ type: "changed", what: "projects" });
    return { id: proj.id, name };
  }
  // CMD-01: a canned explanation, after a short wait as if a model wrote it.
  async explainStep(stepId: string, _lang: string) {
    const a = actions.find((x) => x.id === stepId);
    if (!a) throw new Error("That step is gone.");
    await new Promise((r) => setTimeout(r, 300));
    const cmd = String(a.tool?.command ?? a.tool?.tool ?? "");
    const text = demoExplain.verbose
      ? `- \`${cmd.split(" ")[0]}\`: the program that runs.\n- The rest are its arguments.\n\n**Changes:** see the risk tag; nothing outside the folder.`
      : `Runs \`${cmd}\` in the project folder and prints what it finds. It **changes nothing** on its own.`;
    return { text, cached: false, model: "deepseek-chat" };
  }
  async setOverseer(change: { overseerName?: string; overseerInterject?: boolean }) {
    if (change.overseerName !== undefined) overseerPrefs.name = change.overseerName.trim() || "Overseer";
    if (change.overseerInterject !== undefined) overseerPrefs.interject = change.overseerInterject;
  }
  async overseerTasks(b: { project: string; projectId: string | null; tasks: { title: string; description: string }[] }) {
    let projectId = b.projectId ?? projects.find((p) => p.name.toLowerCase() === b.project.toLowerCase())?.id;
    if (!projectId) {
      projectId = id("p");
      projects.unshift({ id: projectId, name: b.project, description: "", updatedAt: new Date().toISOString() });
      this.emit({ type: "changed", what: "projects" });
    }
    for (const t of b.tasks) await this.createTask({ projectId, title: t.title, description: t.description });
    this.emit({ type: "changed", what: "tasks" });
    return { projectId };
  }
  async interject() {}
  async setCardStyle() {}
  async getNotifications() { return structuredClone(demoNotify); }
  async setNotifications(p: import("./types").NotificationPrefs) {
    if (!!p.quietFrom !== !!p.quietTo) throw new Error("Quiet hours need a start and an end, like 22:00 and 07:00.");
    Object.assign(demoNotify, p);
  }
  async watchMachines() {}
  async pairMachine(name: string) { return { id: "demo", name, token: "demo-token" }; }
  async pairCode() { return { code: "DEMO-C0DE", expiresAt: new Date(Date.now() + 900_000).toISOString() }; }
  async unpairMachine() {}
  async updateRunner() {}
  async listGrants(machineId: string) { return structuredClone(grants[machineId] ?? []); }
  async addGrant(machineId: string, target: string, rights: string[], expiresHours?: number) {
    setTimeout(() => {
      const list = (grants[machineId] ??= []).filter((g) => g.target !== target);
      list.push({ target, rights, grantedBy: "demo", grantedAt: new Date().toISOString(),
        expires: expiresHours ? new Date(Date.now() + expiresHours * 3_600_000).toISOString() : null });
      grants[machineId] = list;
      history.unshift({ at: new Date().toISOString(), kind: "granted", target, detail: rights.join(", "), machine: machineId });
      this.emit({ type: "changed", what: "access", machineId });
    }, 400);
  }
  async revokeGrant(machineId: string, target: string) {
    setTimeout(() => {
      grants[machineId] = (grants[machineId] ?? []).filter((g) => g.target !== target);
      history.unshift({ at: new Date().toISOString(), kind: "revoked", target, detail: null, machine: machineId });
      this.emit({ type: "changed", what: "access", machineId });
    }, 400);
  }
  async renewGrant(machineId: string, target: string, expiresHours: number | null, rights?: string[]) {
    setTimeout(() => {
      const g = (grants[machineId] ?? []).find((x) => x.target === target);
      if (!g) return;
      const was = g.expires;
      if (rights?.length) g.rights = rights;
      g.expires = expiresHours ? new Date(Date.now() + expiresHours * 3_600_000).toISOString() : null;
      g.grantedAt = new Date().toISOString();
      history.unshift({ at: g.grantedAt, kind: "renewed", target, detail: `${was ?? "permanent"} -> ${g.expires ?? "permanent"}`, machine: machineId });
      this.emit({ type: "changed", what: "access", machineId });
    }, 300);
  }
  async accessHistory() { return structuredClone(history); }
  async getAdmin() {
    // NOTIFY: the demo's mail server comes from the environment, as on kireserver.
    return { settings: structuredClone(adminSettings), smtpPasswordSet: true,
      smtpFromEnv: { host: "mailserver", port: 587, tls: "starttls", user: "info@kreative-kompas.com", from: "Kreative Kompanion <kompanion@kreative-kompas.com>" } };
  }
  async webPushKey() { return { key: "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4" }; }
  async webPushSubscribe() {}
  async webPushUnsubscribe() {}
  async saveAdmin(s: AdminSettings) { Object.assign(adminSettings, s); return structuredClone(adminSettings); }
  async uploadLogo() {}
  async removeLogo() {}
  async testMail() { throw new Error("The demo can't send mail."); }
  async login() {}
  async listProjects() { return structuredClone(projects); }
  async setProjectSettings(projectId: string, change: import("./types").ProjectSettingsChange) {
    const p = projects.find((x) => x.id === projectId);
    if (!p) throw new Error("No such project.");
    if (change.repoFolder !== undefined && change.repoFolder && !change.repoFolder.startsWith("/")) {
      throw new Error("Use an absolute folder, like /home/you/projects/app.");
    }
    if ((change.name !== undefined || change.description !== undefined) && p.kind === "windshift") {
      throw new Error("This project is synced with Windshift: change its name and description there.");
    }
    if (change.name !== undefined) {
      const n = change.name.trim();
      if (n.length < 1 || n.length > 80) throw new Error("A name has 1 to 80 characters.");
    }
    if (change.description !== undefined) {
      const d = change.description.trim();
      if (d.length > 200 || /\r|\n/.test(change.description)) throw new Error("The description is one line of at most 200 characters.");
    }
    if (change.about !== undefined) {
      if (change.about.length > 20000) throw new Error("The about text is at most 20,000 bytes.");
    }
    if (change.links !== undefined) {
      if (change.links.length > 10) throw new Error("At most 10 links.");
      for (const l of change.links) {
        const label = l.label?.trim() ?? "";
        if (label.length < 1 || label.length > 60) throw new Error("A link needs a label of 1 to 60 characters.");
        const url = l.url.trim();
        if (!/^https?:\/\/\S+$/.test(url) || url.length > 500) throw new Error("Links must be http or https addresses.");
      }
    }
    if (change.type) p.type = change.type;
    if (change.repoFolder !== undefined) p.repoFolder = change.repoFolder.replace(/\/+$/, "") || null;
    if (change.repoMachineId !== undefined) p.repoMachineId = change.repoMachineId || null;
    if (change.overseer !== undefined) p.overseer = change.overseer;
    if (change.autoRun !== undefined) p.autoRun = change.autoRun;
    if (change.archived !== undefined) p.archivedAt = change.archived ? (p.archivedAt ?? new Date().toISOString()) : null;
    if (change.name !== undefined) p.name = change.name.trim();
    if (change.description !== undefined) p.description = change.description.trim();
    const overviewTextMap = overviewText[projectId] ?? { about: "", links: [] };
    if (change.about !== undefined) overviewTextMap.about = change.about;
    if (change.links !== undefined) overviewTextMap.links = change.links.map((l) => ({ label: l.label.trim(), url: l.url.trim() }));
    overviewText[projectId] = overviewTextMap;
    setTimeout(() => this.emit({ type: "changed", what: "projects" }), 50);
  }
  async getOverview(projectId: string): Promise<import("./types").ProjectOverview> {
    const p = projects.find((x) => x.id === projectId);
    if (!p) throw new Error("No such project.");
    const text = overviewText[projectId] ?? { about: "", links: [] };
    const m = p.repoMachineId ? machines.find((x) => x.id === p.repoMachineId) : undefined;
    const folder = p.repoFolder ?? null;
    const where = {
      machine: m ? { id: m.id, name: m.name, online: m.online, os: m.os || null } : null,
      folder,
      git: m ? { remote: `https://git.kreative-kompas.com/khyretos/${p.name}`, branch: "main", at: ago(5) } : null,
      gitState: m ? ("known" as const) : ("none" as const),
    };
    const covering = m && folder
      ? (grants[m.id] ?? []).filter((g) => g.target.startsWith("/") && (folder === g.target || folder.startsWith(g.target + "/"))).map((g) => ({ target: g.target, rights: g.rights, expires: g.expires }))
      : [];
    const dev = devSettings[projectId] ?? emptyDev();
    const commands = m
      ? (["build", "test", "run", "deploy"] as const).filter((k) => dev[k].trim()).map((k) => ({ kind: k, command: dev[k], last: dev.last[k] ?? null, fromHere: k !== "deploy" }))
      : [];
    const access = { grants: covering, mcp: [{ server: "Blender", tool: "*", expires: hoursFromNow(48) }], knowledge: [], assets: (projectAssets[projectId] ?? []).length, commands };
    const mine = tasks.filter((t) => t.projectId === projectId);
    const progress = {
      done: mine.filter((t) => t.state === "done").length,
      total: mine.length,
      running: mine.filter((t) => t.state === "running").length,
      needsYou: mine.filter((t) => t.state === "needs_input" || t.state === "failed").length,
      inReview: mine.filter((t) => t.state === "in_review").length,
      goals: [],
      next: mine.filter((t) => t.state === "queued").slice(0, 3).map((t) => ({ id: t.id, title: t.title, ready: true })),
      lastMerged: null,
    };
    const activity = [
      ...mine.flatMap((t) => t.events.map((e) => ({ what: `${t.title}: ${"text" in e ? e.text : e.kind}`, at: e.at, who: t.model }))),
      ...chats.filter((c) => c.projectId === projectId).map((c) => ({ what: `Chat: ${c.title}`, at: c.updatedAt, who: "" })),
    ].sort((a, b) => b.at.localeCompare(a.at)).slice(0, 10);
    const ai = mine.length ? [{ model: "Qwen3.5-9B", runs: 4, tokens: 182000, minutes: 12.5 }, { model: "DeepSeek V3", runs: 1, tokens: 24000, minutes: 1.2 }] : [];
    return structuredClone({
      id: p.id,
      name: p.name,
      description: p.description,
      about: text.about,
      type: p.type ?? "chat",
      kind: p.kind ?? "internal",
      source: null,
      links: text.links,
      createdAt: ago(60 * 24 * 30),
      updatedAt: p.updatedAt,
      archivedAt: p.archivedAt ?? null,
      owner: p.ownerName ?? "kees",
      role: p.role ?? "owner",
      members: structuredClone(members[projectId] ?? []),
      where,
      access,
      progress,
      activity,
      ai,
      automation: { scheduler: { enabled: false, budget: { minutes: null, rounds: null, tokens: null } }, overseer: !!p.overseer },
      errors: {},
    });
  }
  async addMember(projectId: string, name: string, role: "viewer" | "editor") {
    const userId = DEMO_USERS[name.trim()];
    if (!userId) throw new Error("No user with that name.");
    const list = members[projectId] ?? (members[projectId] = []);
    const found = list.find((m) => m.userId === userId);
    if (found) found.role = role;
    else list.push({ userId, name: name.trim(), role, you: false });
    setTimeout(() => this.emit({ type: "changed", what: "projects" }), 50);
  }
  async removeMember(projectId: string, userId: string) {
    const left = (members[projectId] ?? []).find((m) => m.userId === userId)?.you;
    members[projectId] = (members[projectId] ?? []).filter((m) => m.userId !== userId);
    if (left) {
      const at = projects.findIndex((p) => p.id === projectId);
      if (at >= 0) projects.splice(at, 1);
      for (let i = chats.length - 1; i >= 0; i--) if (chats[i].projectId === projectId) chats.splice(i, 1);
    }
    setTimeout(() => this.emit({ type: "changed", what: "projects" }), 50);
  }
  async getDevSettings(projectId: string) {
    const p = projects.find((x) => x.id === projectId);
    if (!p) throw new Error("No such project.");
    return structuredClone({ settings: devSettings[projectId] ?? emptyDev(), presets: DEV_PRESETS, repoMachineId: p.repoMachineId ?? null, repoFolder: p.repoFolder ?? null });
  }
  async putDevSettings(projectId: string, s: Omit<DevSettings, "last">) {
    for (const c of [s.build, s.test, s.run, s.deploy]) if (c.length > 500 || /[\n\0]/.test(c)) throw new Error("A command is one line of at most 500 characters.");
    devSettings[projectId] = { ...s, last: devSettings[projectId]?.last ?? {} };
    setTimeout(() => this.emit({ type: "changed", what: "projects" }), 50);
  }
  async runDevCommand(projectId: string, kind: "build" | "test" | "run") {
    const d = devSettings[projectId];
    if (!d || !d[kind].trim()) throw new Error(`Set a ${kind} command first.`);
    setTimeout(() => {
      d.last[kind] = { ok: true, at: new Date().toISOString(), summary: "passed" };
      this.emit({ type: "changed", what: "projects" });
    }, 300);
  }
  async projectAssets(projectId: string) { return structuredClone(projectAssets[projectId] ?? []); }
  async attachAsset(projectId: string, assetId: number) {
    const list = (projectAssets[projectId] ??= []);
    if (!list.some((a) => a.id === assetId)) {
      list.unshift({ id: assetId, name: `asset-${assetId}`, category: "", pack: "", preview: null, pv: 0, missing: false });
    }
    // No "project-assets" event: the demo doesn't know asset names, and the app has
    // already shown the change (the real server sends it for other open tabs).
  }
  async detachAsset(projectId: string, assetId: number) {
    projectAssets[projectId] = (projectAssets[projectId] ?? []).filter((a) => a.id !== assetId);
  }
  async search(q: string): Promise<SearchResult[]> {
    const words = q.toLowerCase().split(/[^\p{L}\p{N}]+/u).filter(Boolean);
    if (!words.length) return [];
    const hit = (text: string) => words.every((w) => new RegExp(`(^|[^\\p{L}\\p{N}])${w.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}`, "iu").test(text));
    const mark = (text: string) => text.replace(new RegExp(`(${words.map((w) => w.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join("|")})`, "giu"), "\u0002$1\u0003");
    const out: SearchResult[] = [];
    for (const t of tasks) { if (hit(t.title + " " + (t.description ?? ""))) out.push({ kind: "task", id: t.id, parent: t.projectId, title: t.title, snippet: mark(t.description ?? "") }); }
    for (const p of projects) { if (hit(p.name + " " + p.description)) out.push({ kind: "project", id: p.id, parent: null, title: p.name, snippet: mark(p.description) }); }
    for (const c of chats) { if (hit(c.title)) out.push({ kind: "chat", id: c.id, parent: null, title: c.title, snippet: "" }); }
    for (const m of messages) { const chat = chats.find((x) => x.id === m.chatId)?.title ?? ""; if (hit(m.text + " " + chat)) out.push({ kind: "message", id: m.id, parent: m.chatId, title: chat, snippet: mark(m.text.slice(0, 160)) }); }
    return out.slice(0, 25);
  }
  async listChats() {
    return structuredClone([...chats].sort((a, b) => Number(!!b.pinned) - Number(!!a.pinned)));
  }
  async listMessages(chatId: string) { return structuredClone(messages.filter((m) => m.chatId === chatId)); }
  async listFacts(where: { chatId?: string; projectId?: string }): Promise<FactsView> {
    const chat = where.chatId ? chats.find((c) => c.id === where.chatId) : undefined;
    const projectId = chat ? chat.projectId ?? null : where.projectId ?? null;
    const view: FactsView = { projectId, chatId: chat?.id ?? null };
    if (projectId) Object.assign(view, { project: sortFacts(facts.filter((f) => f.projectId === projectId)), canEditProject: true });
    if (chat) Object.assign(view, { chat: sortFacts(facts.filter((f) => f.chatId === chat.id)), canEditChat: true });
    return structuredClone(view);
  }
  async addFact(f: { scope: "project" | "chat"; projectId?: string; chatId?: string; key: string; value: string; kind?: FactKind }): Promise<Fact> {
    const key = f.key.trim().replace(/\s+/g, " ");
    const value = f.value.trim();
    if (!key || !value) throw new Error("A fact needs a name and a value.");
    if (/\b(ghp_|sk-|github_pat_)\w{16,}/.test(value)) throw new Error("This looks like a password or token. Facts never hold secrets: write where it lives instead.");
    const scope = f.scope === "project" ? { projectId: f.projectId } : { chatId: f.chatId };
    if (facts.some((x) => (f.scope === "project" ? x.projectId === f.projectId : x.chatId === f.chatId) && x.key.toLowerCase() === key.toLowerCase())) throw new Error(`There is already a fact named "${key}": edit that one instead.`);
    const n = fact(id("f"), scope, key, value, f.kind ?? "text", "user", 0);
    n.lastUsedAt = null;
    facts.push(n);
    this.emit({ type: "changed", what: "facts" });
    return structuredClone(n);
  }
  async updateFact(fid: string, change: { key?: string; value?: string; kind?: FactKind }): Promise<Fact> {
    const f = facts.find((x) => x.id === fid);
    if (!f) throw new Error("That fact is gone.");
    Object.assign(f, { key: change.key?.trim() || f.key, value: change.value?.trim() || f.value, kind: change.kind ?? f.kind, source: "user", updatedAt: new Date().toISOString(), flaggedAt: null });
    this.emit({ type: "changed", what: "facts" });
    return structuredClone(f);
  }
  async deleteFact(fid: string) {
    const i = facts.findIndex((x) => x.id === fid);
    if (i >= 0) facts.splice(i, 1);
    this.emit({ type: "changed", what: "facts" });
  }
  async chatContext(chatId: string): Promise<ChatContext> {
    return structuredClone(contexts[chatId] ?? { chatId, used: null, window: null, model: null, summary: null });
  }
  async compactChat(chatId: string): Promise<ChatContext> {
    const list = messages.filter((m) => m.chatId === chatId);
    if (list.length <= 2) throw new Error("There is nothing new to summarise yet.");
    await wait(600);
    const c = contexts[chatId] ?? { chatId, used: null, window: 128_000, model: "deepseek-chat", summary: null };
    const before = c.summary?.messages ?? 0;
    c.summary = { messages: before + list.length - 2, at: new Date().toISOString(), text: (c.summary ? c.summary.text + "\n" : "") + "- Discussed: " + list.slice(0, -2).map((m) => m.text.split("\n")[0].slice(0, 60)).join("; ") };
    c.used = c.used === null ? null : Math.round(c.used * 0.45);
    contexts[chatId] = c;
    this.emit({ type: "context", usage: structuredClone(c) });
    return structuredClone(c);
  }
  async listTasks(projectId?: string) {
    return structuredClone(projectId ? tasks.filter((t) => t.projectId === projectId) : tasks);
  }
  async listProviders() { return structuredClone(providers); }
  async listMachines() { return structuredClone(machines); }
  async today(): Promise<DaySummary> {
    return { tasks: 41, localTokens: 2_140_000, cloudTokens: 182_000, cloudCostEur: 0.09, energyKwh: 1.4 };
  }
  async listRoles() { return structuredClone(roles); }
  async setRole(a: RoleAssignment) {
    roles = roles.map((r) => (r.role === a.role ? a : r));
  }

  async openThread(projectId: string) {
    let c = chats.find((x) => x.projectId === projectId && x.thread);
    if (!c) {
      c = { id: id("c"), title: "Project thread", projectId, updatedAt: new Date().toISOString(), thread: true };
      chats.unshift(c);
      this.emit({ type: "changed", what: "chats" });
    }
    return c.id;
  }

  async createChat(title: string, projectId?: string) {
    const chat: Chat = { id: id("c"), title, projectId, updatedAt: new Date().toISOString() };
    chats.unshift(chat);
    return structuredClone(chat);
  }

  async updateChat(chatId: string, change: { title?: string; pinned?: boolean; archived?: boolean; projectId?: string; effort?: import("./types").Effort; mcp?: string[]; overseer?: boolean; autoRun?: boolean; machineId?: string }) {
    const i = chats.findIndex((c) => c.id === chatId);
    if (i < 0) return;
    if (change.archived) chats.splice(i, 1);
    else Object.assign(chats[i], { title: change.title ?? chats[i].title, pinned: change.pinned ?? chats[i].pinned, effort: change.effort ?? chats[i].effort, mcp: change.mcp ?? chats[i].mcp, overseer: change.overseer ?? chats[i].overseer, autoRun: change.autoRun ?? chats[i].autoRun, machineId: change.machineId ?? chats[i].machineId });
  }

  async createTask(t: { projectId: string; title: string; description: string; state?: TaskState; chatId?: string; effort?: import("./types").Effort } & import("./types").TaskTreeChange) {
    const task: Task = { id: id("t"), projectId: t.projectId, title: t.title, description: t.description, state: t.state ?? "queued",
      progress: 0, step: "", role: "worker", model: "", events: [],
      effort: t.effort ?? chats.find((c) => c.id === t.chatId)?.effort ?? "auto",
      kind: t.kind ?? "task", parentId: t.parentId || null, dependsOn: t.dependsOn ?? [] };
    tasks.unshift(task);
    return structuredClone(task);
  }
  async updateTask(taskId: string, change: { title?: string; description?: string; state?: TaskState; effort?: import("./types").Effort } & import("./types").TaskTreeChange) {
    const t = tasks.find((x) => x.id === taskId);
    if (!t) throw new Error("No such task.");
    Object.assign(t, Object.fromEntries(Object.entries(change).filter(([, v]) => v !== undefined)));
    if (change.parentId === "") t.parentId = null;
    return structuredClone(t);
  }
  async deleteTask(taskId: string) {
    const i = tasks.findIndex((x) => x.id === taskId);
    // KG-01: like the server, its subtasks move up to its own parent.
    if (i >= 0) for (const c of tasks) if (c.parentId === taskId) c.parentId = tasks[i].parentId ?? null;
    for (const c of tasks) if (c.dependsOn?.includes(taskId)) c.dependsOn = c.dependsOn.filter((d) => d !== taskId);
    if (i >= 0) tasks.splice(i, 1);
  }
  async reorderTasks(_projectId: string, ids: string[]) {
    ids.forEach((tid, i) => { const t = tasks.find((x) => x.id === tid); if (t) t.position = i; });
  }
  async makeProjectInternal() {}
  async deleteChat(chatId: string) {
    const i = chats.findIndex((c) => c.id === chatId);
    if (i >= 0) chats.splice(i, 1);
  }

  /** Demo timeline: an hour of samples every 30 s, two jobs and a role switch on the A770. */
  async gpuTimeline(hours: 1 | 24) {
    const end = Date.now(), span = hours * 3600_000, step = hours === 1 ? 30_000 : 240_000;
    const at = (ms: number) => new Date(end - ms).toISOString();
    const samples = Array.from({ length: Math.floor(span / step) }, (_, k) => {
      const t = span - k * step, busy = t < 1_200_000 && t > 600_000;
      return { at: at(t), usedMib: busy ? 14800 : 12100, reservedMib: 15400, watts: busy ? 168 : 42 };
    });
    return [
      { gpu: "a770", machine: "kireserver", hours, samples,
        jobs: [{ id: "j1", kind: "code", what: "W2 run: initials()", state: "done", startedAt: at(1_200_000), endedAt: at(600_000), error: null },
          { id: "j2", kind: "asset", what: "studio:comfyui:oc sheet", state: "running", startedAt: at(240_000), endedAt: null, error: null }],
        events: [{ at: at(300_000), kind: "role", detail: "coder → artist (comfyui), 3.0 s" }] },
      { gpu: "a580", machine: "kireserver", hours, samples: samples.map((s) => ({ ...s, usedMib: 900, reservedMib: 0, watts: 12 })), jobs: [], events: [] },
    ];
  }
  async studioTypes() {
    return [
      { name: "character", label: "Character", hint: "Full body, clean background", sizes: ["tall", "square"], order: 1 },
      { name: "scene", label: "Scene", hint: "Characters in a place", sizes: ["wide", "square", "tall"], order: 2 },
      { name: "landscape", label: "Landscape", hint: "Wide place, no people", sizes: ["wide", "square", "tall"], order: 3 },
      { name: "sprite", label: "Sprite", hint: "One game object, plain background", sizes: ["square", "tall", "wide"], order: 4 },
      { name: "icon", label: "Icon", hint: "Simple and bold, for menus", sizes: ["square"], order: 5 },
      { name: "oc-sheet", label: "OC sheet", hint: "Character turnaround", sizes: ["wide"], order: 6, warning: "novaAnimeXL_ilV170.safetensors: Fair AI Public License 1.0-SD: unknown licence, check it before use", ratings: ["general", "sensitive", "questionable", "explicit"], adultRatings: ["questionable", "explicit"], face: true, negative: "", helmet: true },
      { name: "video", label: "Video", hint: "A few seconds, no sound", sizes: ["wide", "tall", "square"], order: 7 },
      ...AUDIO_TYPES,
    ];
  }
  async studioMake(type: string, prompt: string, size: string, count: 1 | 4, _rating?: string, face?: { file: File; weight: number }, _negative?: string, params?: Record<string, string>) {
    if (face && !face.file.type.startsWith("image/")) throw new Error("The photo must be a JPEG, PNG or WebP.");
    // STU-C3: a helmet description makes a pair per picture (helmet on, then the face).
    const desc = params?.helmet_desc?.trim();
    const sides: ("on" | "off" | null)[] = desc ? ["on", "off"] : [null];
    const made = Array.from({ length: count }, (_, i) => {
      const pair = desc ? id("pair") : null;
      return sides.map((helmet) => ({ id: id("sr"), type, gpu: "rx9070", prompt: helmet === "on" ? `${prompt}, ${desc}` : prompt, size, state: "running" as const, error: null, files: [] as string[], startedAt: new Date().toISOString(), endedAt: null as string | null, helmet, pair, hue: 200 + i * 30 + (helmet === "off" ? 90 : 0) }));
    }).flat();
    studioRuns.unshift(...made.map(({ hue: _h, ...r }) => r));
    setTimeout(() => {
      for (const m of made) {
        const r = studioRuns.find((x) => x.id === m.id);
        if (r) Object.assign(r, { state: "done", files: [demoPic(m.hue)], endedAt: new Date().toISOString() });
      }
      this.emit({ type: "changed", what: "studio" });
    }, 1200);
    return { ids: made.map((m) => m.id) };
  }
  async studioMine() { return structuredClone(studioRuns); }
  async studioDelete(ids: string[], failed = false) {
    const gone = studioRuns.filter((r) => r.state !== "running" && (ids.includes(r.id) || (failed && r.state === "failed"))).map((r) => r.id);
    for (const g of gone) studioRuns.splice(studioRuns.findIndex((r) => r.id === g), 1);
    return { deleted: gone, skipped: ids.filter((i) => !gone.includes(i)), keptFiles: 0 };
  }
  async studioToAssets(runId: string, project: string, _n = 0) {
    const r = studioRuns.find((x) => x.id === runId);
    if (!r || r.state !== "done") throw new Error("Only a finished result can go to Assets.");
    return { assetId: 9000, project, name: r.files[0] ?? "" };
  }
  async studioAudio(kind: "music" | "sfx", prompt: string, _lyrics: string, seconds: number) {
    const run = { id: id("sr"), type: kind, gpu: "rx9070", prompt, size: "square", seconds, state: "running" as const, error: null, files: [] as string[], startedAt: new Date().toISOString(), endedAt: null as string | null };
    studioRuns.unshift(run);
    setTimeout(() => {
      const r = studioRuns.find((x) => x.id === run.id);
      if (r) Object.assign(r, { state: "done", files: ["data:audio/ogg;base64,", "data:audio/wav;base64,"], endedAt: new Date().toISOString() });
      this.emit({ type: "changed", what: "studio" });
    }, 1200);
    return { ids: [run.id] };
  }
  async setStudioTarget(target: string) {
    studioTarget = target;
    if (target === "off") { soucouyantMode = "gaming"; soucouyantStopped = true; }
    this.emit({ type: "changed", what: "gpus" });
  }
  async setGpuMode(machine: string, mode: import("../views/capabilities").GpuModeName) {
    if (machine === "soucouyant") {
      soucouyantMode = mode;
      if (mode === "gaming") soucouyantStopped = true;
    }
    this.emit({ type: "changed", what: "gpus" });
  }
  async knowledge() {
    return { collections: structuredClone(knowledgeCols) };
  }
  async createKnowledge(name: string) {
    const c = knowledgeCols.find((k) => k.name === name);
    if (c) return { id: c.id, name: c.name };
    const newCol: import("../views/knowledge").KCollection = { id: `kc-${++seq}`, name, source: "app", docs: 0, chunks: 0, vectors: 0, projects: [], documents: [] };
    knowledgeCols.push(newCol);
    knowledgeCols.sort((a, b) => a.name.localeCompare(b.name));
    return { id: newCol.id, name };
  }
  async deleteKnowledge(id: string) {
    const i = knowledgeCols.findIndex((k) => k.id === id);
    if (i >= 0) knowledgeCols.splice(i, 1);
  }
  async setKnowledgeProjects(id: string, projects: string[]) {
    const c = knowledgeCols.find((k) => k.id === id);
    if (c) c.projects = projects;
  }
  async uploadKnowledge(id: string, file: File, onProgress: (pct: number) => void) {
    const ext = file.name.includes(".") ? file.name.split(".").pop()!.toLowerCase() : "";
    if (!["md", "markdown", "txt", "text", "html", "htm", "pdf", "rst", "csv", "json"].includes(ext)) throw new Error(`unsupported file type: ${ext}`);
    const c = knowledgeCols.find((k) => k.id === id);
    if (!c) throw new Error("Not found.");
    onProgress(50);
    await new Promise((r) => setTimeout(r, 150));
    onProgress(100);
    const doc = { id: ++seq, name: file.name, chars: file.size, addedAt: new Date().toISOString() };
    c.documents.unshift(doc);
    c.docs += 1;
    c.chunks += 1;
    return { id: doc.id, name: doc.name, chunks: 1 };
  }
  async knowledgeDocs(id: string, q: string, offset: number) {
    const all = (knowledgeCols.find((k) => k.id === id)?.documents ?? []).filter((d) => d.name.toLowerCase().includes(q.toLowerCase())).sort((a, b) => a.name.localeCompare(b.name));
    return { documents: all.slice(offset, offset + 50), total: all.length };
  }
  async knowledgeDoc(id: string, doc: number) {
    const d = knowledgeCols.find((k) => k.id === id)?.documents.find((x) => x.id === doc);
    if (!d) throw new Error("Not found.");
    return { id: d.id, name: d.name, text: `# ${d.name}\n\nThe text of this document, as the app stored it.` };
  }
  async deleteKnowledgeDoc(id: string, doc: number) {
    const c = knowledgeCols.find((k) => k.id === id);
    if (c) {
      const i = c.documents.findIndex((d) => d.id === doc);
      if (i >= 0) {
        c.documents.splice(i, 1);
        c.docs = Math.max(0, c.docs - 1);
        c.chunks = Math.max(0, c.chunks - 1);
      }
    }
  }
  async getCapabilities() {
    return {
      gpus: [
        { id: "a770", machine: "kireserver", totalMib: 16384, usedMib: 13210, reservedMib: 15400, otherMib: 0, freeMib: 984, schedulable: true,
          holdings: [{ name: "Coder", kind: "model", nowMib: 11700, peakMib: 11700, busy: false },
            { name: "Whisper", kind: "model", nowMib: 1700, peakMib: 1700, busy: false }] },
        { id: "a580", machine: "kireserver", totalMib: 8192, usedMib: 900, reservedMib: 0, otherMib: 900, freeMib: 7292, schedulable: false, holdings: [] },
        { id: "rx9070", machine: "soucouyant", totalMib: 16304, usedMib: soucouyantStopped ? 400 : 6900, reservedMib: soucouyantStopped ? 0 : 6355, otherMib: 400, freeMib: soucouyantStopped ? 15904 : 9549, schedulable: true,
          holdings: soucouyantStopped ? [] : [{ name: "ComfyUI (soucouyant)", kind: "app", nowMib: 6355, peakMib: 6355, busy: false }] },
      ],
      workflows: [
        { name: "z-image-turbo", title: "Z-Image Turbo: text to image", description: "Fast 8-step image from a prompt.", studio: null, base: null,
          models: [{ file: "z-image-turbo-Q8_0.gguf", licence: "Apache-2.0" }, { file: "ae.safetensors", licence: "Apache-2.0" }], problems: [], runnable: true,
          targets: ["a770", "rx9070"], runs: 12, lastRun: ago(20), avgSeconds: 23.4 },
        { name: "landscape", title: "Landscape", description: "A wide place without people.", studio: { label: "Landscape" }, base: "z-image-turbo",
          models: [{ file: "z-image-turbo-Q8_0.gguf", licence: "Apache-2.0" }], problems: [], runnable: true, targets: ["a770", "rx9070"], runs: 0, lastRun: null, avgSeconds: null },
        { name: "sd15-sprites", title: "Old SD 1.5 sprites", description: "Kept to show a licence warning.", studio: null, base: null,
          models: [{ file: "sd15.ckpt", licence: "CreativeML OpenRAIL-M" }], problems: ["sd15.ckpt: CreativeML OpenRAIL-M: OpenRAIL licence, check its use restrictions"], runnable: true,
          targets: ["a770", "rx9070"], runs: 0, lastRun: null, avgSeconds: null },
      ],
      studioTarget: { target: studioTarget, queued: 0, choices: [
        { value: "auto", label: "Automatic", cost: "The studio computer while its studio is on, else the fallback GPU" },
        { value: "a770", label: "kireserver (a770)", cost: "Coder pauses while it runs" },
        { value: "rx9070", label: "soucouyant (rx9070)", cost: "" },
        { value: "off", label: "Off", cost: "No studio jobs; the studio apps are stopped" }] },
      gpuModes: [{ machine: "soucouyant", mode: soucouyantMode, effective: soucouyantMode, gaming: false, appsStopped: soucouyantStopped,
        granted: true, studioAt: ago(4), gpus: ["rx9070"], apps: ["comfyui", "heartmula", "sfx"] }],
      models: [
        { id: "ovms", name: "OVMS on kireserver", local: true, status: "ok" as const, error: null, models: ["Coder", "Whisper"],
          roles: ["orchestrator: Coder", "reviewer: Coder", "worker: Coder"], lastError: null },
        { id: "ollama-soucouyant", name: "Ollama on soucouyant", local: true, status: "down" as const, error: "connection refused",
          models: [], roles: [], lastError: { text: "connection refused", at: ago(42) } },
      ],
      // Live from the demo's own machines and grants, so the page follows changes made elsewhere.
      computers: machines.filter((m) => m.id !== "kireserver").map((m) => ({
        id: m.id, name: m.name, online: m.online, lastSeen: new Date().toISOString(),
        grants: (grants[m.id] ?? []).map((g) => ({ target: g.target, rights: [...g.rights], expires: g.expires })),
      })),
      tools: [
        { name: "read_file", description: "Read a text file (up to 256 KiB).", needs: "read on the folder" },
        { name: "edit_file", description: "Replace one exact piece of a file and show the diff.", needs: "write on the folder" },
        { name: "shell", description: "Run a command in a folder.", needs: "shell in the folder" },
        { name: "capabilities", description: "What Kompanion can use right now.", needs: "nothing" },
        // CAP-02: long names and text, so the overflow test has something to wrap and clamp.
        { name: "gpu_apps_status_start_stop_unload_ollama_and_free_the_vram", description: "Start, stop or check a GPU app by its container name (ComfyUI, HeartMuLa, the sound effects app) on a computer with the GPU right, and unload Ollama's models first when a job needs the VRAM: /home/kees/.config/comfyui-rocm/compose.override.yml", needs: "gpu on the computer" },
      ],
      // CHAT-01: two tool servers in the demo, one of them down.
      mcp: [
        { name: "Stack Overflow", status: "ok" as const, description: "3.5 million answered questions", tools: [
          { name: "search_stackoverflow", description: "Search Stack Overflow questions" }, { name: "get_post", description: "Read a question and its answers" }] },
        { name: "Developer docs", status: "down" as const, description: null, error: "Developer docs answered 401 Unauthorized", tools: [] },
      ],
      indexes: [{ id: "assets", name: "Asset search by meaning", items: 41_230, of: 47_012, failed: 12, model: "Embedder (ovms-cpu)", status: "partly" as const }],
      skills: [
        { id: "work-habits", title: "Work habits", lessons: 14, updated: ago(60), layer: "general" as const, file: "work-habits.md" },
        { id: "orchestrator", title: "Orchestrator", lessons: 9, updated: ago(30), layer: "kompanion" as const, file: "orchestrator/SKILL.md" },
        { id: "worker/rust", title: "Worker: Rust", lessons: 37, updated: ago(5), layer: "kompanion" as const, file: "worker/rust/SKILL.md" },
        { id: "worker/web", title: "Worker: web app (vanilla TypeScript)", lessons: 25, updated: ago(90), layer: "kompanion" as const, file: "worker/web/SKILL.md" },
        { id: "shared/colour-themes", title: "Colour themes (this setup)", lessons: 3, updated: ago(200), layer: "private" as const, file: "shared/colour-themes.md" },
        { id: "shared/theming-brand-kreative-kompas-palette-contrast-and-fonts", title: "Theming: the Kreative Kompas palette, contrast ratios and fonts", lessons: 4, updated: ago(300), layer: "private" as const, file: "shared/theming-brand.md" },
      ],
    };
  }
  async voiceInfo() {
    return { enabled: true, voices: [{ id: "af_heart", label: "English (US), Heart" }, { id: "ef_dora", label: "Spanish, Dora" }] };
  }
  async transcribe(audio: Blob) {
    await new Promise((r) => setTimeout(r, 300));
    return audio.size >= 0 ? "install htop" : "";
  }
  /** 1.5 s of silence per sentence, so the demo "reads" without sound (long enough to stop it in a test). */
  async speak() {
    await new Promise((r) => setTimeout(r, 100));
    const rate = 8000, n = rate * 1.5, b = new DataView(new ArrayBuffer(44 + n * 2));
    const put = (o: number, s: string) => { for (let i = 0; i < s.length; i++) b.setUint8(o + i, s.charCodeAt(i)); };
    put(0, "RIFF"); b.setUint32(4, 36 + n * 2, true); put(8, "WAVEfmt "); b.setUint32(16, 16, true); b.setUint16(20, 1, true);
    b.setUint16(22, 1, true); b.setUint32(24, rate, true); b.setUint32(28, rate * 2, true); b.setUint16(32, 2, true);
    b.setUint16(34, 16, true); put(36, "data"); b.setUint32(40, n * 2, true);
    return new Blob([b.buffer], { type: "audio/wav" });
  }
  async skillHistory(_layer: string, file: string) {
    return { commits: [{ sha: "a1b2c3d", message: `skills: edit ${file}`, author: "kees", date: ago(60 * 26) }] };
  }

  async saveSkill(_layer: string, _file: string, _text: string, _message: string) { return { commit: "c0ffee1" }; }

  async moveLesson(_from: string, _to: string, _file: string, _line: string) { return { to: "c0ffee2", from: "c0ffee3" }; }

  async getSkill(id: string, layer?: string) {
    return { id, layer: layer ?? "kompanion", text: `# ${id}\n\nLessons for this role, newest last.\n\n1. (2026-10-04) Plan steps are changes, each with a **done when**.\n2. Never \`test.skip\` inside a test.\n` };
  }

  async listActions(chatId: string) { return structuredClone(actions.filter((a) => (a as PcAction & { chatId?: string }).chatId === chatId)); }
  async listLessons(chatId: string) { return structuredClone(lessons.filter((l) => l.chatId === chatId)); }
  async decideLesson(lessonId: string, decision: "accept" | "dismiss", text?: string, layer?: "general" | "private") {
    const l = lessons.find((x) => x.id === lessonId);
    if (!l || l.state !== "proposed") throw new Error("This lesson was already decided.");
    l.state = decision === "accept" ? "accepted" : "dismissed";
    if (layer) l.layer = layer;
    if (decision === "accept" && text?.trim()) l.text = text.trim();
    const m: Message = { id: id("m"), chatId: l.chatId, author: "orchestrator", at: new Date().toISOString(),
      text: decision === "accept" ? `Lesson added to \`${l.card}\` (${l.layer}, /data/skills/${l.card}.md).` : `Lesson for \`${l.card}\` dismissed.` };
    messages.push(m);
    this.emit({ type: "message", message: structuredClone(m) });
    this.emit({ type: "changed", what: "lessons" });
  }
  async listActivity() {
    const now = Date.now();
    return [
      { at: new Date(now - 120_000).toISOString(), kind: "step", state: "done", text: "Edit hyprland.lua", machineId: "soucouyant", machine: "soucouyant",
        chatId: "c1", chat: "Hyprland gaps", id: "a1", tool: { tool: "edit_file", path: "/home/kees/.config/hypr/hyprland.lua" },
        result: "@@ line 12 @@\n-gaps_out = 8\n+gaps_out = 12\n", grant: "allowed once · 10 min · removed after" },
      { at: new Date(now - 300_000).toISOString(), kind: "step", state: "refused", text: "Read /etc/shadow", machineId: "soucouyant", machine: "soucouyant",
        chatId: "c1", chat: "Hyprland gaps", id: "a2", tool: { tool: "read_file", path: "/etc/shadow" }, result: "not granted: read on /etc/shadow", grant: null },
      { at: new Date(now - 600_000).toISOString(), kind: "grant", text: "system", target: "system", detail: "always allow, 24 h", machineId: "soucouyant", machine: "soucouyant" },
    ];
  }
  async startTask(taskId: string, machineId: string, _folder?: string, _check?: string, _tests?: boolean, effort?: import("./types").Effort) {
    const t = tasks.find((x) => x.id === taskId);
    if (!t) return;
    if (effort) t.effort = effort;
    t.state = "running"; t.step = "planning";
    this.emit({ type: "task", task: structuredClone(t) });
    const thread = await this.openThread(t.projectId);
    const pc = machines.find((m) => m.id === machineId)?.name ?? machineId;
    const post = (ms: number, text: string, then?: () => void) => setTimeout(() => {
      const m: Message = { id: id("m"), chatId: thread, author: "orchestrator", at: new Date().toISOString(), text };
      messages.push(m);
      this.emit({ type: "message", message: structuredClone(m) });
      then?.();
    }, ms);
    post(300, `Started **${t.title}** on ${pc}.`);
    post(900, `**${t.title}**, step 1/2 done: added the timer around the shader compile.`);
    post(1500, `**${t.title}**, step 2/2 done: the times go to the console.`);
    post(1800, `**${t.title}**, review round 1: 1 finding(s). [Open the task](#task=${t.id})`, () => {
      lessons.push({ id: id("l"), chatId: thread, card: "worker/cpp-games/SKILL", text: "Time a compile step with std::chrono::steady_clock, never system_clock.",
        finding: "The timer used system_clock, which jumps when the clock changes.", state: "proposed", createdAt: new Date().toISOString(), layer: "general" as const, proposedBy: "coder" });
      this.emit({ type: "changed", what: "lessons" });
    });
    post(2100, `Done: **${t.title}** (done after 1 round(s)). [Open the task](#task=${t.id})`, () => {
      t.state = "done"; t.step = "done after 1 round(s)"; t.progress = 1;
      this.emit({ type: "task", task: structuredClone(t) });
    });
  }
  async taskCosts(taskId: string) {
    return taskId === "t-kk-1"
      ? { coder: { jobs: 6, output: 6446, prompt: 18801, gpu_seconds: 216.3, lines: 330 }, claude: { answers: 46, output: 38592, input: 92, cache_read: 33617601, cache_write: 735453 }, output_share_coder: 0.14 }
      : null;
  }
  async weeklyCosts() { return { tasks: 5, coderOutput: 53100, claudeOutput: 301600, coderShare: 0.15 }; }
  async taskRuns(taskId: string) {
    if (taskId === "t-kk-1") return [{ id: "run-demo-1", startedAt: ago(30), endedAt: ago(29), status: "needs_input", step: "folder not found", effort: "high", effortPicked: true }];
    // TASK-OVERVIEW: a running task has a running run in the demo.
    const t = tasks.find((x) => x.id === taskId);
    return t?.state === "running" ? [{ id: `run-${taskId}`, startedAt: ago(12), endedAt: null, status: "running", step: t.step ?? null, effort: "auto", effortPicked: false }] : [];
  }
  // TASK-OVERVIEW: a demo trace: two model calls around a build step that is still running.
  async runTrace(runId: string): Promise<import("./types").RunTrace> {
    // Tests call this to act as a live update of the open trace (as a new step would).
    (globalThis as { __kkDemoTraceTick?: () => void }).__kkDemoTraceTick = () => this.emit({ type: "changed", what: "trace" });
    const t = tasks.find((x) => `run-${x.id}` === runId);
    if (!t) throw new Error("That run is gone.");
    const at = (min: number) => new Date(Date.now() - min * 60_000).toISOString();
    const call = (id: string, min: number, tin: number, tout: number, ms: number, prompt: string, response: string): import("./types").TraceCall =>
      ({ kind: "call", id, at: at(min), role: "worker", provider: "ovms", model: "Qwen3.5-9B", tokensIn: tin, tokensOut: tout, ms, error: null, reason: "A step on a computer.", prompt, response });
    const items: import("./types").RunTrace["items"] = [
      call(`${runId}-c1`, 11, 6_200, 310, 4_100, '[\n  {"role": "user", "content": "Add a --vk-validation flag (off in release builds) and an engine.toml setting."}\n]', '{"tool_calls": [{"function": {"name": "read_file", "arguments": "{\\"path\\": \\"/home/kees/projects/kk-engine/src/config.cpp\\"}"}}]}'),
      { kind: "step", id: `${runId}-s1`, machineId: "soucouyant", summary: "Read /home/kees/projects/kk-engine/src/config.cpp", state: "done", result: "// engine config\nstruct Config { bool validation = false; };", createdAt: at(10), startedAt: at(10), endedAt: at(10), tool: { tool: "read_file", path: "/home/kees/projects/kk-engine/src/config.cpp" } },
      call(`${runId}-c2`, 9, 7_050, 880, 9_300, '[\n  {"role": "tool", "content": "// engine config\\nstruct Config { bool validation = false; };"}\n]', '{"content": "I will build and run the tests.", "tool_calls": [{"function": {"name": "shell"}}]}'),
      { kind: "step", id: `${runId}-s2`, machineId: "soucouyant", summary: t.step ?? "Build and test", state: "running", result: null, createdAt: at(8), startedAt: at(8), endedAt: null, tool: { tool: "shell", cwd: "/home/kees/projects/kk-engine", command: "cmake --build build -j && ctest --test-dir build" } },
    ];
    return { run: { id: runId, taskId: t.id, title: t.title, project: projects.find((p) => p.id === t.projectId)?.name ?? null, status: "running", startedAt: at(12), endedAt: null }, nowDoing: t.step ?? "", items };
  }
  async stopAction(actionId: string) {
    const a = actions.find((x) => x.id === actionId);
    if (!a || a.state !== "running") throw new Error("That step isn't running.");
    setTimeout(() => { a.state = "stopped"; a.result = "stopped by you\nexit: -1"; this.emit({ type: "changed", what: "actions" }); }, 200);
  }
  async stopTask(taskId: string) {
    const t = tasks.find((x) => x.id === taskId);
    if (!t || t.state !== "running") throw new Error("This task isn't running.");
    setTimeout(() => { t.state = "needs_input"; t.step = "stopped by you"; this.emit({ type: "task", task: structuredClone(t) }); }, 200);
  }
  /** Demo: there is no running reply loop to stop, so this does nothing. */
  async stopChat(_chatId: string) { /* nothing to stop in the demo */ }
  /** Demo computers: /home/kees and its projects exist, everything else is missing. */
  async checkFolder(_machineId: string, path: string) {
    await new Promise((r) => setTimeout(r, 150));
    const tree: Record<string, string[]> = { "/": ["home"], "/home": ["kees"], "/home/kees": ["projects", "notes"], "/home/kees/projects": ["kk-engine", "kompanion"],
      "/home/kees/projects/kk-engine": ["src", "games"], "/home/kees/projects/kompanion": ["server", "web"], "/home/kees/notes": [] };
    const p = path.replace(/\/+$/, "") || "/";
    return p in tree ? { state: "ok" as const, path: p, folders: tree[p], files: 3 }
      : { state: "missing" as const, path: p, message: `Folder not found on soucouyant: ${p}` };
  }
  async decideAction(actionId: string, decision: "approve" | "always" | "deny") {
    const a = actions.find((x) => x.id === actionId);
    if (!a || a.state !== "pending") throw new Error("This step was already decided.");
    const step = (state: PcAction["state"], result: string | null, ms: number) =>
      setTimeout(() => { a.state = state; a.result = result; this.emit({ type: "changed", what: "actions" }); }, ms);
    if (decision === "deny") { step("denied", null, 0); return; }
    // AGENT-SPAWN: a task in the chat's project (with the prompt as its description), as the server does.
    if (a.tool?.tool === "start_work" && a.tool.kind === "task") {
      const from = chats.find((c) => c.id === (a as PcAction & { chatId?: string }).chatId);
      if (!from?.projectId) { step("failed", "failed: This chat is not in a project, so there is no task list: start a chat instead.", 0); return; }
      const task: Task = { id: id("t"), projectId: from.projectId, title: String(a.tool.title), description: String(a.tool.prompt), state: "queued", progress: 0, step: "Waiting to start", role: "worker", model: "", events: [] };
      tasks.unshift(task);
      this.emit({ type: "task", task: structuredClone(task) });
      step("running", null, 0);
      step("done", `Added the task "${task.title}" to this project (task=${task.id}). It waits in the task list until it is started.`, 300);
      return;
    }
    // AGENT-SPAWN: a new chat in the same project, with the prompt as its first message.
    if (a.tool?.tool === "start_work") {
      const from = chats.find((c) => c.id === (a as PcAction & { chatId?: string }).chatId);
      const chat: Chat = { id: id("c"), title: String(a.tool.title), projectId: from?.projectId, updatedAt: new Date().toISOString() };
      chats.unshift(chat);
      messages.push({ id: id("m"), chatId: chat.id, author: "user", at: new Date().toISOString(), text: String(a.tool.prompt) });
      this.emit({ type: "changed", what: "chats" });
      step("running", null, 0);
      step("done", `Started the chat "${chat.title}" (chat=${chat.id}); it is answering the prompt there now.`, 300);
      return;
    }
    step("granting", null, 0);
    if (decision === "always") {
      // One grant per target, like the runner (ACC-01: the demo already has a system grant).
      grants[a.machineId] = (grants[a.machineId] ?? []).filter((g) => g.target !== "system");
      grants[a.machineId].push({ target: "system", rights: ["packages", "root"], grantedBy: "demo",
        grantedAt: new Date().toISOString(), expires: new Date(Date.now() + 86_400_000).toISOString() });
      this.emit({ type: "changed", what: "access", machineId: a.machineId });
    }
    step("running", null, 300);
    step("done", "resolving dependencies...\ninstalling htop...\nexit: 0", 600);
  }
  async attach(_chatId: string, file: File) {
    const kind = kindOf(file.name);
    return { name: file.name, file: `${id("f")}.${kind === "image" ? "png" : kind === "file" ? "bin" : "txt"}`, kind } as import("../core/attachments").Attached;
  }
  async send(chatId: string, text: string, machineId?: string, _effort?: unknown, attachments: import("../core/attachments").Attached[] = []) {
    if (attachments.length) text += "\n\n:::files\n" + JSON.stringify(attachments);
    if (machineId && /long step/i.test(text)) {
      // Demo of a long command (20 s) that can be stopped.
      const a = { id: id("a"), chatId, machineId, summary: "Run `cargo build` in /home/kees/projects/kk-engine", tool: { tool: "shell", cwd: "/home/kees/projects/kk-engine", command: "cargo build" },
        state: "running", result: null, createdAt: new Date(Date.now() + 50).toISOString() } as PcAction & { chatId: string };
      actions.push(a);
      setTimeout(() => this.emit({ type: "changed", what: "actions" }), 100);
      setTimeout(() => { if (a.state !== "running") return; a.state = "done"; a.result = "Finished\nexit: 0"; this.emit({ type: "changed", what: "actions" }); }, 20_000);
    } else if (machineId && /command cards/i.test(text)) {
      // CMD-01: finished steps of each risk, and one still running, as command cards.
      // Created after the question, so all four sit under the reply; the times are what ran.
      const t0 = Date.now() + 20;
      const done: [string, Record<string, unknown>, string, number][] = [
        ["Look for TODOs in src", { tool: "shell", cwd: "/home/kees/projects/kk-engine", command: "grep -rn \"TODO\" src | head -20" }, "done", 2],
        ["Clean the build folder", { tool: "shell", cwd: "/home/kees/projects/kk-engine", command: "rm -rf build/ && mkdir build" }, "done", 1],
        ["Fetch the latest changes", { tool: "shell", cwd: "/home/kees/projects/kk-engine", command: "git pull --rebase origin main" }, "failed", 4],
      ];
      done.forEach(([summary, tool, state, secs], k) => actions.push({ id: id("a"), chatId, machineId, summary, tool, state, result: state === "done" ? "src/render.cpp:42: // TODO: batch draws\nexit: 0" : "fatal: could not read from remote\nexit: 1",
        createdAt: new Date(t0 + k).toISOString(), startedAt: new Date(t0 - 60_000 + k * 10_000).toISOString(), endedAt: new Date(t0 - 60_000 + k * 10_000 + secs * 1000).toISOString() } as PcAction & { chatId: string }));
      actions.push({ id: id("a"), chatId, machineId, summary: "Build and test", tool: { tool: "shell", cwd: "/home/kees/projects/kk-engine", command: "cmake --build build -j && ctest --test-dir build --output-on-failure" },
        state: "running", result: null, createdAt: new Date(t0 + 3).toISOString(), startedAt: new Date(Date.now() - 12_000).toISOString() } as PcAction & { chatId: string });
      setTimeout(() => this.emit({ type: "changed", what: "actions" }), 100);
    } else if (machineId && /hand this on as a task/i.test(text)) {
      // AGENT-SPAWN: the model asks to add a task to this chat's project.
      actions.push({ id: id("a"), chatId, machineId, summary: "Add the task \"Write the battle rules\"",
        tool: { tool: "start_work", kind: "task", title: "Write the battle rules", prompt: "Goal: turns, parts and damage on one page.\n1. List the parts.\nDone when two bots can fight on paper." },
        state: "pending", result: null, createdAt: new Date(Date.now() + 50).toISOString() } as PcAction & { chatId: string });
      setTimeout(() => this.emit({ type: "changed", what: "actions" }), 100);
    } else if (machineId && /hand this on/i.test(text)) {
      // AGENT-SPAWN: the model asks to start a new chat; the card waits for Approve or Deny.
      actions.push({ id: id("a"), chatId, machineId, summary: "Start a new chat: \"Logo ideas for the shop\"",
        tool: { tool: "start_work", kind: "chat", title: "Logo ideas for the shop", prompt: "Sketch three logo ideas for the shop: round, with the compass mark, readable at 32 px.\nSay which one you would pick and why." },
        state: "pending", result: null, createdAt: new Date(Date.now() + 50).toISOString() } as PcAction & { chatId: string });
      setTimeout(() => this.emit({ type: "changed", what: "actions" }), 100);
    } else if (machineId && /artifact demo/i.test(text)) {
      // ARTIFACTS: a file the model wrote, with Open on its card.
      actions.push({ id: id("a"), chatId, machineId, summary: "Write `src/battle.ts`", tool: { tool: "write_file", path: "/home/kees/projects/medabots/src/battle.ts", content: ARTIFACT_TS },
        state: "done", result: `wrote ${ARTIFACT_TS.length} characters`, createdAt: new Date(Date.now() + 20).toISOString(), startedAt: new Date(Date.now() - 2000).toISOString(), endedAt: new Date(Date.now() - 1500).toISOString() } as PcAction & { chatId: string });
      setTimeout(() => this.emit({ type: "changed", what: "actions" }), 100);
    } else if (machineId && /every card type/i.test(text)) {
      // Demo of every approval card type (readability checks): one pending card each.
      const tools: [string, Record<string, unknown>][] = [
        ["Read `Cargo.toml`", { tool: "read_file", path: "/home/kees/projects/kk-engine/Cargo.toml" }],
        ["Edit `src/main.rs`", { tool: "edit_file", path: "/home/kees/projects/kk-engine/src/main.rs" }],
        ["Run `cargo test` in kk-engine", { tool: "shell", command: "cargo test" }],
        ["Download the asset pack", { tool: "shell", command: "curl -LO https://example.org/pack.zip" }],
        ["install htop with paru", { tool: "package", manager: "paru", action: "install", names: ["htop"] }],
        ["Run `git diff --stat` in kk-engine", { tool: "shell", command: "git diff --stat" }],
      ];
      tools.forEach(([summary, tool], k) => actions.push({ id: id("a"), chatId, machineId, summary, tool, needs: k === 4 ? "packages + root" : undefined,
        state: "pending", result: null, createdAt: new Date(Date.now() + k).toISOString() } as PcAction & { chatId: string }));
      setTimeout(() => this.emit({ type: "changed", what: "actions" }), 100);
    } else if (machineId && /three steps/i.test(text)) {
      // Demo of a step group: three automatic steps that finish one after another.
      const at = Date.now() + 50;
      for (let k = 0; k < 3; k++) {
        const a = { id: id("a"), chatId, machineId, summary: `step ${k + 1}: read file ${k + 1}`, tool: { tool: "read_file", path: `/home/kees/f${k + 1}.txt` },
          state: k === 0 ? "running" : "approved", result: null, createdAt: new Date(at + k).toISOString() } as PcAction & { chatId: string };
        actions.push(a);
        setTimeout(() => { a.state = "running"; this.emit({ type: "changed", what: "actions" }); }, 600 + k * 900);
        setTimeout(() => { if (a.state === "stopped") return; a.state = "done"; a.result = `line ${k + 1}`; this.emit({ type: "changed", what: "actions" }); }, 1000 + k * 900);
      }
      setTimeout(() => this.emit({ type: "changed", what: "actions" }), 100);
    } else if (machineId) {
      const a = { id: id("a"), chatId, machineId, summary: "install htop with paru", tool: { tool: "package", manager: "paru", action: "install", names: ["htop"] }, needs: "packages + root (asks for the password on the PC)",
        state: "pending", result: null, createdAt: new Date().toISOString() } as PcAction & { chatId: string };
      actions.push(a);
      setTimeout(() => this.emit({ type: "changed", what: "actions" }), 100);
    }
    const user: Message = { id: id("m"), chatId, author: "user", text, at: new Date().toISOString() };
    messages.push(user);
    this.emit({ type: "message", message: structuredClone(user) });
    // OVR-01: a canned Overseer answer: a status, two proposed tasks and context for a running task.
    if (chats.find((c) => c.id === chatId)?.overseer) {
      const proposal = { tasks: [
        { project: "Medabots", projectId: null, title: "Write the battle rules", description: "Turns, parts and damage on one page; done when two bots can fight on paper." },
        { project: "Medabots", projectId: null, title: "Sketch three robot personalities", description: "Name, temper and fighting style for each." },
      ], interjections: [{ taskId: "t1", title: "Add a --vk-validation flag and setting", text: "Kees wants the flag off by default in release builds.", sent: overseerPrefs.interject }] };
      const reply: Message = { id: id("m"), chatId, author: "orchestrator", at: new Date().toISOString(), text:
        "**Needs you:** nothing right now.\n\n**Running:** **Add a --vk-validation flag and setting** (62%), building in the container.\n\n**Next:** two queued tasks in kk-engine.\n\nFor Medabots I would start small:\n" +
        "TASK: Medabots | Write the battle rules | Turns, parts and damage on one page; done when two bots can fight on paper.\nTASK: Medabots | Sketch three robot personalities | Name, temper and fighting style for each.\nINTERJECT: T1 | Kees wants the flag off by default in release builds." +
        "\n\n:::overseer\n" + JSON.stringify(proposal) };
      setTimeout(() => { messages.push(reply); this.emit({ type: "message", message: structuredClone(reply) }); }, 300);
      return;
    }

    // A project thread: "pause" and "go on" steer the project's running tasks (server: thread::command).
    const thread = chats.find((c) => c.id === chatId && c.thread);
    const word = text.trim().toLowerCase().replace(/[^\p{L}\p{N}\s]/gu, "").split(/\s+/).join(" ");
    const cmd = ["pause", "pause please", "hold on", "wait", "stop for now"].includes(word) ? "pause"
      : ["go on", "continue", "resume", "go ahead", "carry on"].includes(word) ? "resume" : undefined;
    if (thread && cmd) {
      const running = tasks.filter((t) => t.projectId === thread.projectId && t.state === "running");
      const titles = running.map((t) => `**${t.title}**`).join(", ");
      const answer = !running.length ? "Nothing is running in this project."
        : cmd === "pause" ? `Paused ${titles}: the current step finishes, then it waits. Write "go on" to continue.`
        : `Going on with ${titles}.`;
      const m: Message = { id: id("m"), chatId, author: "orchestrator", text: answer, at: new Date().toISOString() };
      messages.push(m);
      setTimeout(() => this.emit({ type: "message", message: structuredClone(m) }), 200);
      return;
    }

    const chat = chats.find((c) => c.id === chatId);
    const reply: Message = { id: id("m"), chatId, author: "orchestrator", text: "", at: new Date().toISOString(), streaming: true };
    messages.push(reply);
    await wait(500);
    this.emit({ type: "message", message: structuredClone(reply) });

    const full = /artifact demo/i.test(text) ? ARTIFACT_REPLY : replies[this.replyIndex++ % replies.length];
    for (const word of full.split(/(?<=\s)/)) {
      await wait(35);
      reply.text += word;
      this.emit({ type: "message-delta", messageId: reply.id, chatId, text: word, done: false });
    }
    reply.streaming = false;
    this.emit({ type: "message-delta", messageId: reply.id, chatId, text: "", done: true });
    // CTX-01: the meter grows by what this exchange added (about four characters a token).
    const ctx = contexts[chatId] ?? { chatId, used: 2_400, window: 128_000, model: "deepseek-chat", summary: null };
    ctx.used = (ctx.used ?? 2_400) + Math.round((text.length + full.length) / 4);
    contexts[chatId] = ctx;
    this.emit({ type: "context", usage: structuredClone(ctx) });

    if (chat?.projectId) {
      const task: Task = {
        id: id("t"), projectId: chat.projectId, title: text.length > 60 ? text.slice(0, 57) + "…" : text,
        state: "running", progress: 0.05, step: "Reading the code map", role: "worker", model: "Qwen3.5-9B",
        activeRun: true, lastActivity: new Date().toISOString(),
        runner: "soucouyant (Linux)", workspace: "Container · 4 cores · 8 GB",
        events: [{ kind: "step", at: new Date().toISOString(), text: "Planned by the orchestrator" }],
      };
      tasks.unshift(task);
      reply.taskIds = [task.id];
      this.emit({ type: "task", task: structuredClone(task) });
    }
  }

  // MSG-ACT: the user's message answered again (with new text when given); what followed goes.
  async resend(chatId: string, messageId: string, text?: string) {
    const list = messages.filter((m) => m.chatId === chatId);
    const i = list.findIndex((m) => m.id === messageId);
    if (i < 0 || list[i].author !== "user") throw new Error("Only your own messages can be sent again.");
    if (text !== undefined) {
      const files = list[i].text.split("\n\n:::files\n")[1];
      list[i].text = text.trim() + (files ? `\n\n:::files\n${files}` : "");
    }
    const drop = new Set(list.slice(i + 1).map((m) => m.id));
    for (let k = messages.length - 1; k >= 0; k--) if (drop.has(messages[k].id)) messages.splice(k, 1);
    setTimeout(() => void this.streamReply(chatId), 0);
    return structuredClone(messages.filter((m) => m.chatId === chatId));
  }
  private async streamReply(chatId: string) {
    const reply: Message = { id: id("m"), chatId, author: "orchestrator", text: "", at: new Date().toISOString(), streaming: true };
    messages.push(reply);
    await wait(300);
    this.emit({ type: "message", message: structuredClone(reply) });
    const full = replies[this.replyIndex++ % replies.length];
    for (const word of full.split(/(?<=\s)/)) {
      await wait(20);
      reply.text += word;
      this.emit({ type: "message-delta", messageId: reply.id, chatId, text: word, done: false });
    }
    reply.streaming = false;
    this.emit({ type: "message-delta", messageId: reply.id, chatId, text: "", done: true });
  }

  async answer(taskId: string, optionId: string) {
    const t = tasks.find((x) => x.id === taskId);
    if (!t || !t.question) return;
    const option = t.question.options.find((o) => o.id === optionId);
    t.events.push({ kind: "step", at: new Date().toISOString(), text: `You chose: ${option?.label ?? optionId}` });
    t.question = undefined;
    if (optionId === "reject") {
      t.state = "failed";
      t.step = "You denied the command";
    } else if (optionId === "tonight") {
      t.state = "queued";
      t.step = "Scheduled for 01:00";
      t.scheduledFor = nextAt(1);
    } else {
      t.state = "running";
      t.step = "Starting";
    }
    this.emit({ type: "task", task: structuredClone(t) });
  }

  onEvent(listener: (ev: ServerEvent) => void) {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }
}

function clamp(n: number) {
  return Math.min(0.98, Math.max(0.02, n));
}

function wait(ms: number) {
  return new Promise((r) => setTimeout(r, ms));
}

function nextAt(hour: number): string {
  const d = new Date();
  d.setHours(hour, 0, 0, 0);
  if (d.getTime() < Date.now()) d.setDate(d.getDate() + 1);
  return d.toISOString();
}
