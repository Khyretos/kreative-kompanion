// CMD-01: what a step card shows about its command: the command as typed, its language for the
// highlighter, and a quick risk tag guessed from the tool and the words in the command.
import type { Key } from "./i18n";

export type Risk = "read" | "write" | "network" | "delete";

export const RISK_LABEL: Record<Risk, Key> = { read: "riskRead", write: "riskWrite", network: "riskNetwork", delete: "riskDelete" };

type Tool = Record<string, unknown> | undefined;
const str = (v: unknown): string => (typeof v === "string" ? v : "");

/** AGENT-SPAWN: a step that starts a new chat or task (answered by Kompanion, not a computer). */
export const SPAWN = "start_work";
export function spawnOf(tool: Tool): { kind: "chat" | "task"; title: string; prompt: string } | undefined {
  if (str(tool?.tool) !== SPAWN) return undefined;
  return { kind: str(tool?.kind) === "task" ? "task" : "chat", title: str(tool?.title), prompt: str(tool?.prompt) };
}

/** What a finished spawn step made, from its result ("… (chat=<id>) …"), for an Open button. */
export function spawnedRef(result: string | null): { kind: "chat" | "task"; id: string } | undefined {
  const m = /\((chat|task)=([\w-]+)\)/.exec(result ?? "");
  return m ? { kind: m[1] as "chat" | "task", id: m[2] } : undefined;
}

/** The command a step runs, as a person would type it (the server's explain::command_text). A
 *  spawn step has none: its card shows the prompt instead. */
export function commandText(tool: Tool): string {
  const t = tool ?? {};
  switch (str(t.tool)) {
    case SPAWN:
      return "";
    case "shell": {
      const [cwd, cmd] = [str(t.cwd), str(t.command)];
      return cwd ? `cd ${cwd} && ${cmd}` : cmd;
    }
    case "package":
      return `${str(t.manager)} ${str(t.action)} ${(Array.isArray(t.names) ? t.names.map(String) : []).join(" ")}`.trim();
    case "service":
      return `systemctl ${str(t.action)} ${str(t.unit)}`.trim();
    case "read_file":
      return `read ${str(t.path)}`;
    case "list_dir":
      return `ls ${str(t.path)}`;
    case "write_file": {
      const how = t.append === true ? "append to" : typeof t.offset === "number" ? `write at byte ${t.offset} of` : "write";
      return `${how} ${str(t.path)} (${str(t.content).length} characters)`;
    }
    case "edit_file":
      return `edit ${str(t.path)}`;
    case "system_info":
      return `system_info ${str(t.what)}`.trim();
    default:
      return tool ? JSON.stringify(tool) : "";
  }
}

/** Shell-like steps are highlighted as bash; the rest stay plain. */
export function commandLang(tool: Tool): string {
  return ["shell", "package", "service"].includes(str(tool?.tool)) ? "bash" : "text";
}

const DELETES = /(^|[\s;&|(])(rm|rmdir|shred|unlink|wipefs|mkfs(\.\w+)?|dd)\s|\bgit\s+(clean|reset\s+--hard|branch\s+-D|push\s+\S*\s*--force|push\s+-f)\b|\bdocker\s+(rm|rmi|volume\s+rm|system\s+prune|image\s+prune)\b|\bfind\b.*\s-delete\b|\btruncate\s|\bDROP\s+(TABLE|DATABASE)\b|\bdel\s|\bRemove-Item\b/i;
const NETWORK = /\b(curl|wget|ssh|scp|rsync|sftp|ftp|ping|nc|ncat|telnet|dig|nslookup|Invoke-WebRequest|iwr)\b|\bgit\s+(clone|fetch|pull|push|ls-remote)\b|\b(apt(-get)?|dnf|yum|pacman|brew|winget|choco)\s+(install|update|upgrade|-S)\b|\b(npm|pnpm|yarn)\s+(install|i|add|ci|publish)\b|\bpip3?\s+install\b|\bcargo\s+(install|fetch|publish)\b|\bdocker\s+(pull|push|login)\b|https?:\/\//i;
const READ_ONLY = /^(cd|ls|ll|dir|cat|head|tail|less|more|grep|rg|ag|find|fd|tree|wc|stat|file|du|df|free|uptime|uname|whoami|id|pwd|echo|printf|which|type|env|printenv|date|ps|top|htop|lsblk|lscpu|lspci|lsusb|ip|ss|netstat|journalctl|systemctl\s+(status|list-units|is-active|show)|git\s+(status|log|diff|show|branch|remote|rev-parse|blame)|docker\s+(ps|images|logs|inspect|stats)|nvidia-smi|sensors|Get-\w+|true|test|\[)\b/;

/** A quick guess at what a step does to the computer: deletes beats network beats changes files.
 *  None for a step that does not touch a computer (a new chat or task). */
export function riskOf(tool: Tool): Risk | undefined {
  const name = str(tool?.tool);
  if (name === SPAWN) return undefined;
  if (["read_file", "list_dir", "system_info"].includes(name)) return "read";
  if (name === "write_file" || name === "edit_file") return "write";
  if (name === "package") return /^(remove|purge|uninstall)$/.test(str(tool?.action)) ? "delete" : "network";
  if (name === "service") return "write";
  if (name !== "shell") return "write";
  const cmd = str(tool?.command).trim();
  if (DELETES.test(cmd)) return "delete";
  if (NETWORK.test(cmd)) return "network";
  // Read only when every part of a pipeline or list is a known reader and nothing is written to a file.
  const parts = cmd.split(/&&|\|\||;|\|/).map((p) => p.trim()).filter(Boolean);
  const writesFile = /(^|[^0-9&])>{1,2}\s*(?!&|\/dev\/null)\S/.test(cmd) || /\btee\b|\bsed\s+-i\b/.test(cmd);
  return parts.length > 0 && !writesFile && parts.every((p) => READ_ONLY.test(p.replace(/^sudo\s+/, ""))) ? "read" : "write";
}
