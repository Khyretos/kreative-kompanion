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
  // PROV-UI: model providers in Settings, and New project.
  provHead: { en: "Connected models", es: "Modelos conectados", nl: "Verbonden modellen" },
  provAdd: { en: "Add a provider", es: "Añadir un proveedor", nl: "Provider toevoegen" },
  provAddButton: { en: "Add provider", es: "Añadir proveedor", nl: "Provider toevoegen" },
  provEdit: { en: "Change {name}", es: "Cambiar {name}", nl: "{name} wijzigen" },
  provEditButton: { en: "Change", es: "Cambiar", nl: "Wijzigen" },
  provRemove: { en: "Remove", es: "Quitar", nl: "Verwijderen" },
  provRemoveAsk: { en: "Remove the provider {name}? Its models leave every list.", es: "¿Quitar el proveedor {name}? Sus modelos desaparecen de todas las listas.", nl: "Provider {name} verwijderen? Zijn modellen verdwijnen uit alle lijsten." },
  provSave: { en: "Save", es: "Guardar", nl: "Opslaan" },
  provPreset: { en: "Start from", es: "Empezar desde", nl: "Beginnen met" },
  provPresetPick: { en: "Pick a provider…", es: "Elige un proveedor…", nl: "Kies een provider…" },
  provPresetOther: { en: "Other (OpenAI-compatible)", es: "Otro (compatible con OpenAI)", nl: "Andere (OpenAI-compatibel)" },
  provName: { en: "Name", es: "Nombre", nl: "Naam" },
  provKind: { en: "API", es: "API", nl: "API" },
  provKindOpenai: { en: "OpenAI-compatible", es: "Compatible con OpenAI", nl: "OpenAI-compatibel" },
  provKindAnthropic: { en: "Anthropic (Claude)", es: "Anthropic (Claude)", nl: "Anthropic (Claude)" },
  provUrl: { en: "Address", es: "Dirección", nl: "Adres" },
  provLocal: { en: "Runs on my own network (local)", es: "Funciona en mi propia red (local)", nl: "Draait op mijn eigen netwerk (lokaal)" },
  provKey: { en: "Where the key is", es: "Dónde está la clave", nl: "Waar de sleutel staat" },
  provKeyEnv: { en: "Environment variable", es: "Variable de entorno", nl: "Omgevingsvariabele" },
  provKeyFile: { en: "Secret file", es: "Archivo secreto", nl: "Geheim bestand" },
  provKeyNone: { en: "No key", es: "Sin clave", nl: "Geen sleutel" },
  provEnvName: { en: "Variable name", es: "Nombre de la variable", nl: "Naam van de variabele" },
  provEnvHelp: { en: "Set it in the server's .env (in capitals, ending in _KEY or _TOKEN) and restart Kompanion.", es: "Ponla en el .env del servidor (en mayúsculas, terminada en _KEY o _TOKEN) y reinicia Kompanion.", nl: "Zet hem in de .env van de server (in hoofdletters, eindigend op _KEY of _TOKEN) en herstart Kompanion." },
  provFilePath: { en: "File path", es: "Ruta del archivo", nl: "Pad van het bestand" },
  provFileHelp: { en: "A file in {dirs}, read when it is used.", es: "Un archivo en {dirs}, que se lee al usarlo.", nl: "Een bestand in {dirs}, gelezen wanneer het nodig is." },
  provMaxTokens: { en: "Output tokens per answer", es: "Tokens de salida por respuesta", nl: "Uitvoertokens per antwoord" },
  provWindow: { en: "Context window (tokens)", es: "Ventana de contexto (tokens)", nl: "Contextvenster (tokens)" },
  provDefault: { en: "default", es: "por defecto", nl: "standaard" },
  provLookUp: { en: "look it up", es: "buscarla", nl: "opzoeken" },
  provNeverShown: { en: "Kompanion never shows or stores the key itself, only where it is.", es: "Kompanion nunca muestra ni guarda la clave, solo dónde está.", nl: "Kompanion toont of bewaart de sleutel nooit, alleen waar hij staat." },
  provKeyFromEnv: { en: "key from {name}", es: "clave de {name}", nl: "sleutel uit {name}" },
  provKeyFromFile: { en: "key from {path}", es: "clave de {path}", nl: "sleutel uit {path}" },
  provNoKey: { en: "no key", es: "sin clave", nl: "geen sleutel" },
  provKeyFound: { en: "found", es: "encontrada", nl: "gevonden" },
  provKeyMissing: { en: "not found yet", es: "aún no encontrada", nl: "nog niet gevonden" },
  provFromToml: { en: "Set in kompanion.toml", es: "Definido en kompanion.toml", nl: "Ingesteld in kompanion.toml" },
  provAdminOnly: { en: "An admin adds and changes providers.", es: "Un administrador añade y cambia los proveedores.", nl: "Een beheerder voegt providers toe en wijzigt ze." },
  projNew: { en: "New project", es: "Proyecto nuevo", nl: "Nieuw project" },
  projName: { en: "Project name", es: "Nombre del proyecto", nl: "Projectnaam" },
  projType: { en: "Type", es: "Tipo", nl: "Type" },
  projTypeChat: { en: "Chat", es: "Chat", nl: "Chat" },
  projTypeGame: { en: "Game", es: "Juego", nl: "Game" },
  projTypeProgramming: { en: "Programming", es: "Programación", nl: "Programmeren" },
  projCreate: { en: "Create", es: "Crear", nl: "Maken" },
  // MSG-ACT: actions under a message.
  msgActions: { en: "Message actions", es: "Acciones del mensaje", nl: "Berichtacties" },
  msgCopy: { en: "Copy message", es: "Copiar mensaje", nl: "Bericht kopiëren" },
  msgCopied: { en: "Copied", es: "Copiado", nl: "Gekopieerd" },
  msgRegen: { en: "Regenerate this answer", es: "Volver a generar esta respuesta", nl: "Dit antwoord opnieuw maken" },
  msgEdit: { en: "Edit and resend", es: "Editar y reenviar", nl: "Bewerken en opnieuw sturen" },
  msgEditLabel: { en: "Edit your message", es: "Edita tu mensaje", nl: "Bewerk je bericht" },
  msgSend: { en: "Send", es: "Enviar", nl: "Versturen" },
  msgCancel: { en: "Cancel", es: "Cancelar", nl: "Annuleren" },
  msgEditNote: { en: "Everything after this message is removed and answered again.", es: "Todo lo que viene después de este mensaje se borra y se responde de nuevo.", nl: "Alles na dit bericht wordt verwijderd en opnieuw beantwoord." },
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
