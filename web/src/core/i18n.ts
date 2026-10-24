// UI strings in English, Spanish and Dutch, picked from the browser's language (English otherwise).
// New UI text goes through t(); the rest of the app is still English only.
export type Lang = "en" | "es" | "nl";

export function lang(): Lang {
  const l = (globalThis.navigator?.language ?? "en").slice(0, 2).toLowerCase();
  return l === "es" || l === "nl" ? l : "en";
}

const STRINGS = {
  export: { en: "Export", es: "Exportar", nl: "Exporteren" },
  exportTip: {
    en: "Export: this chat's steps and model calls as JSON",
    es: "Exportar: los pasos y las llamadas al modelo de este chat en JSON",
    nl: "Exporteren: de stappen en modelaanroepen van deze chat als JSON",
  },
  ctxFull: {
    en: "Context {pct}% full: {used} of {window} tokens. Show details",
    es: "Contexto al {pct}%: {used} de {window} tokens. Ver detalles",
    nl: "Context {pct}% vol: {used} van {window} tokens. Details tonen",
  },
  ctxUnknown: {
    en: "Context: {used} tokens used, the model's window is unknown. Show details",
    es: "Contexto: {used} tokens usados, no se sabe el tamaño de la ventana del modelo. Ver detalles",
    nl: "Context: {used} tokens gebruikt, de venstergrootte van het model is onbekend. Details tonen",
  },
  ctxNone: { en: "Context: no answer yet. Show details", es: "Contexto: aún no hay respuesta. Ver detalles", nl: "Context: nog geen antwoord. Details tonen" },
  ctxDialog: { en: "Context of this chat", es: "Contexto de este chat", nl: "Context van deze chat" },
  ctxHead: { en: "{used} of {window} tokens ({pct}%)", es: "{used} de {window} tokens ({pct}%)", nl: "{used} van {window} tokens ({pct}%)" },
  ctxHeadUnknown: {
    en: "{used} tokens; the window of {model} is unknown (set context_window for its provider).",
    es: "{used} tokens; no se sabe la ventana de {model} (pon context_window en su proveedor).",
    nl: "{used} tokens; het venster van {model} is onbekend (zet context_window bij de provider).",
  },
  ctxNoAnswer: { en: "No answer yet in this chat.", es: "Aún no hay respuesta en este chat.", nl: "Nog geen antwoord in deze chat." },
  ctxModel: { en: "{model}, latest answer", es: "{model}, última respuesta", nl: "{model}, laatste antwoord" },
  ctxWarn: {
    en: "Nearly full: the next answer first summarises the older messages.",
    es: "Casi lleno: la próxima respuesta resume antes los mensajes antiguos.",
    nl: "Bijna vol: het volgende antwoord vat eerst de oudere berichten samen.",
  },
  ctxSummary: { en: "Summary of {n} older messages · {when}", es: "Resumen de {n} mensajes anteriores · {when}", nl: "Samenvatting van {n} oudere berichten · {when}" },
  ctxSummaryOne: { en: "Summary of 1 older message · {when}", es: "Resumen de 1 mensaje anterior · {when}", nl: "Samenvatting van 1 ouder bericht · {when}" },
  ctxNote: {
    en: "Messages that no longer fit are summarised, never dropped silently.",
    es: "Los mensajes que ya no caben se resumen; nunca se pierden sin avisar.",
    nl: "Berichten die niet meer passen worden samengevat, nooit stil weggelaten.",
  },
  ctxCompact: { en: "Summarise now", es: "Resumir ahora", nl: "Nu samenvatten" },
  ctxCompacting: { en: "Summarising…", es: "Resumiendo…", nl: "Samenvatten…" },
  ctxThis: { en: "this model", es: "este modelo", nl: "dit model" },
  // CMD-01: step cards and "Explain command".
  riskRead: { en: "read only", es: "solo lectura", nl: "alleen lezen" },
  riskWrite: { en: "changes files", es: "cambia archivos", nl: "wijzigt bestanden" },
  riskNetwork: { en: "network", es: "red", nl: "netwerk" },
  riskDelete: { en: "deletes", es: "borra", nl: "verwijdert" },
  riskTip: { en: "A quick guess from the command, not a guarantee", es: "Una estimación rápida a partir del comando, no una garantía", nl: "Een snelle inschatting op basis van het commando, geen garantie" },
  stStarting: { en: "starting…", es: "iniciando…", nl: "starten…" },
  stGranting: { en: "waiting for the computer…", es: "esperando al ordenador…", nl: "wachten op de computer…" },
  stRunning: { en: "running…", es: "en curso…", nl: "bezig…" },
  stDenied: { en: "declined", es: "rechazado", nl: "geweigerd" },
  stDone: { en: "done", es: "hecho", nl: "klaar" },
  stFailed: { en: "failed", es: "falló", nl: "mislukt" },
  stRefused: { en: "not allowed", es: "no permitido", nl: "niet toegestaan" },
  stStopped: { en: "stopped by you", es: "parado por ti", nl: "door jou gestopt" },
  stPending: { en: "waiting for you", es: "esperándote", nl: "wacht op jou" },
  cmdCommand: { en: "Command", es: "Comando", nl: "Commando" },
  cmdOutput: { en: "Output", es: "Salida", nl: "Uitvoer" },
  cmdStop: { en: "Stop", es: "Parar", nl: "Stoppen" },
  cmdTook: { en: "took {time}", es: "tardó {time}", nl: "duurde {time}" },
  spawnChat: { en: "New chat", es: "Chat nuevo", nl: "Nieuwe chat" },
  spawnTask: { en: "New task", es: "Tarea nueva", nl: "Nieuwe taak" },
  spawnPrompt: { en: "Prompt", es: "Mensaje", nl: "Prompt" },
  spawnOpenChat: { en: "Open the chat", es: "Abrir el chat", nl: "Chat openen" },
  spawnOpenTask: { en: "Open the task", es: "Abrir la tarea", nl: "Taak openen" },
  spawnBy: { en: "made by Kompanion, no computer involved", es: "lo hace Kompanion, sin ordenador", nl: "door Kompanion, zonder computer" },
  approve: { en: "Approve", es: "Aprobar", nl: "Goedkeuren" },
  deny: { en: "Deny", es: "Rechazar", nl: "Weigeren" },
  explainCmd: { en: "Explain command", es: "Explicar comando", nl: "Commando uitleggen" },
  explaining: { en: "Explaining…", es: "Explicando…", nl: "Uitleggen…" },
  explainFailed: { en: "No explanation: {error}", es: "Sin explicación: {error}", nl: "Geen uitleg: {error}" },
  explainRetry: { en: "Try again", es: "Reintentar", nl: "Opnieuw" },
  explainHead: { en: "Explain running actions", es: "Explicar las acciones en curso", nl: "Lopende acties uitleggen" },
  explainAbout: {
    en: "Step cards get an \"Explain command\" section. The explanation is written when you open it, and kept for the same command.",
    es: "Las tarjetas de pasos tienen una sección \"Explicar comando\". La explicación se escribe al abrirla y se guarda para el mismo comando.",
    nl: "Stapkaarten krijgen een sectie \"Commando uitleggen\". De uitleg wordt geschreven als je die opent en bewaard voor hetzelfde commando.",
  },
  explainDetail: { en: "Detail", es: "Detalle", nl: "Detail" },
  explainCompact: { en: "Compact", es: "Breve", nl: "Kort" },
  explainVerbose: { en: "Verbose", es: "Detallada", nl: "Uitgebreid" },
  explainModel: { en: "Explain with model", es: "Explicar con el modelo", nl: "Uitleggen met model" },
  explainChatModel: { en: "The chat's model", es: "El modelo del chat", nl: "Het model van de chat" },
  explainNote: { en: "Uses extra tokens each time you open an explanation.", es: "Usa tokens extra cada vez que abres una explicación.", nl: "Kost extra tokens telkens als je een uitleg opent." },
} as const;

export type Key = keyof typeof STRINGS;

/** The string for the user's language, with {name} filled from `vars`. */
export function t(key: Key, vars: Record<string, string | number> = {}, l: Lang = lang()): string {
  return STRINGS[key][l].replace(/\{(\w+)\}/g, (m, name: string) => (name in vars ? String(vars[name]) : m));
}

/** A number in the user's language (thousands separators). */
export function num(n: number, l: Lang = lang()): string {
  return n.toLocaleString(l === "es" ? "es-ES" : l === "nl" ? "nl-NL" : "en-US");
}
