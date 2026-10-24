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
  // TASK-OVERVIEW: the running-tasks panel.
  ovChip: { en: "{running} running, {done} of {total} done. Show the overview", es: "{running} en curso, {done} de {total} hechas. Ver el resumen", nl: "{running} bezig, {done} van {total} klaar. Overzicht tonen" },
  ovTitle: { en: "Tasks now", es: "Tareas ahora", nl: "Taken nu" },
  ovClose: { en: "Close the overview", es: "Cerrar el resumen", nl: "Overzicht sluiten" },
  ovRunning: { en: "Running", es: "En curso", nl: "Bezig" },
  ovDone: { en: "Done", es: "Hechas", nl: "Klaar" },
  ovTotal: { en: "Total", es: "Total", nl: "Totaal" },
  ovNeedsYou: { en: "Need you", es: "Te necesitan", nl: "Wachten op jou" },
  ovNone: { en: "Nothing is running right now.", es: "Ahora no hay nada en curso.", nl: "Er loopt nu niets." },
  ovStarting: { en: "starting…", es: "empezando…", nl: "begint…" },
  ovBack: { en: "All running", es: "Todas en curso", nl: "Alle lopende" },
  ovNow: { en: "Now:", es: "Ahora:", nl: "Nu:" },
  ovLoading: { en: "Reading what it did…", es: "Leyendo lo que hizo…", nl: "Lezen wat hij deed…" },
  ovEmpty: { en: "No model calls or steps yet.", es: "Aún no hay llamadas ni pasos.", nl: "Nog geen modelaanroepen of stappen." },
  ovCall: { en: "Model call", es: "Llamada al modelo", nl: "Modelaanroep" },
  ovTokens: { en: "{in} in · {out} out", es: "{in} entrada · {out} salida", nl: "{in} in · {out} uit" },
  ovPrompt: { en: "What it got", es: "Lo que recibió", nl: "Wat hij kreeg" },
  ovAnswer: { en: "What it answered", es: "Lo que respondió", nl: "Wat hij antwoordde" },
  ovFree: { en: "Read from what Kompanion records anyway: no extra tokens.", es: "Sale de lo que Kompanion ya registra: sin tokens extra.", nl: "Komt uit wat Kompanion toch al bijhoudt: geen extra tokens." },
  // NOTIFY: notification settings.
  ntfWhen: { en: "When", es: "Cuándo", nl: "Wanneer" },
  ntfApproval: { en: "A step waits for my approval", es: "Un paso espera mi aprobación", nl: "Een stap wacht op mijn goedkeuring" },
  ntfHow: { en: "How", es: "Cómo", nl: "Hoe" },
  ntfMail: { en: "By mail", es: "Por correo", nl: "Per mail" },
  ntfPush: { en: "As a push to my phone app and the browsers I turned it on in", es: "Como aviso en la app del móvil y en los navegadores donde lo activé", nl: "Als pushmelding op mijn telefoonapp en in de browsers waar ik het aanzette" },
  ntfQuiet: { en: "Quiet hours", es: "Horas de silencio", nl: "Stille uren" },
  ntfFrom: { en: "From", es: "Desde", nl: "Van" },
  ntfTo: { en: "to", es: "hasta", nl: "tot" },
  ntfQuietHelp: { en: "No mail or push in this time (your clock); open apps still show it. Leave both empty for none.", es: "Sin correos ni avisos en este horario (tu hora); las apps abiertas lo siguen mostrando. Deja ambos vacíos para ninguno.", nl: "Geen mail of push in deze tijd (jouw klok); open apps tonen het nog wel. Laat beide leeg voor geen." },
  wpOn: { en: "Push in this browser", es: "Avisos en este navegador", nl: "Pushmeldingen in deze browser" },
  wpOff: { en: "Stop push in this browser", es: "Quitar avisos en este navegador", nl: "Pushmeldingen in deze browser uitzetten" },
  wpStateOn: { en: "On: this browser gets notifications, also when Kompanion is closed.", es: "Activado: este navegador recibe avisos, también con Kompanion cerrado.", nl: "Aan: deze browser krijgt meldingen, ook als Kompanion dicht is." },
  wpStateOff: { en: "Off in this browser.", es: "Desactivado en este navegador.", nl: "Uit in deze browser." },
  wpStateDenied: { en: "Blocked: allow notifications for this site in the browser's settings.", es: "Bloqueado: permite los avisos de este sitio en los ajustes del navegador.", nl: "Geblokkeerd: sta meldingen voor deze site toe in de instellingen van de browser." },
  wpStateNone: { en: "This browser cannot get push notifications (install Kompanion as an app, or use another browser).", es: "Este navegador no puede recibir avisos (instala Kompanion como app o usa otro navegador).", nl: "Deze browser kan geen pushmeldingen krijgen (installeer Kompanion als app of gebruik een andere browser)." },
  wpHelp: { en: "Push reaches this browser through its maker's push service; it carries only the task title, its state and a link.", es: "El aviso llega a este navegador por el servicio de avisos de su fabricante; solo lleva el título, el estado y un enlace.", nl: "De melding komt via de pushdienst van de browsermaker; er staan alleen de titel, de status en een link in." },
  smtpEnv: { en: "Mail goes through {host}:{port} ({tls}) as {from}, user {user}. The server's .env sets this (KOMPANION_SMTP_*), so it can't be changed here.", es: "El correo sale por {host}:{port} ({tls}) como {from}, usuario {user}. Lo fija el .env del servidor (KOMPANION_SMTP_*), así que no se cambia aquí.", nl: "Mail gaat via {host}:{port} ({tls}) als {from}, gebruiker {user}. De .env van de server bepaalt dit (KOMPANION_SMTP_*), dus het kan hier niet worden gewijzigd." },
  smtpPwSet: { en: "The mail password is set in the server's environment (never shown).", es: "La contraseña del correo está en el entorno del servidor (nunca se muestra).", nl: "Het mailwachtwoord staat in de omgeving van de server (wordt nooit getoond)." },
  smtpPwMissing: { en: "No mail password set: add KOMPANION_SMTP_PASSWORD (or a file in KOMPANION_SMTP_PASSWORD_FILE) to the server's .env if your mail server needs one.", es: "Sin contraseña de correo: añade KOMPANION_SMTP_PASSWORD (o un archivo en KOMPANION_SMTP_PASSWORD_FILE) al .env del servidor si tu servidor de correo la necesita.", nl: "Geen mailwachtwoord: zet KOMPANION_SMTP_PASSWORD (of een bestand in KOMPANION_SMTP_PASSWORD_FILE) in de .env van de server als je mailserver er een nodig heeft." },
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
  // EFF-02: the composer's effort chip and picker.
  effAuto: { en: "Auto", es: "Auto", nl: "Auto" },
  effLow: { en: "Low", es: "Baja", nl: "Laag" },
  effMedium: { en: "Medium", es: "Media", nl: "Gemiddeld" },
  effHigh: { en: "High", es: "Alta", nl: "Hoog" },
  effAutoHint: { en: "Medium for now; picks per task later", es: "Media por ahora; luego elige por tarea", nl: "Voorlopig gemiddeld; later per taak gekozen" },
  effLowHint: { en: "Quick answer, fewest tool rounds", es: "Respuesta rápida, menos rondas de herramientas", nl: "Snel antwoord, minste toolrondes" },
  effMediumHint: { en: "Balanced: a few tool rounds", es: "Equilibrada: unas pocas rondas de herramientas", nl: "Gebalanceerd: een paar toolrondes" },
  effHighHint: { en: "Thinks first, most tool rounds", es: "Piensa antes, más rondas de herramientas", nl: "Denkt eerst na, meeste toolrondes" },
  effChip: { en: "Effort {effort}, model {model}. Change effort or model", es: "Esfuerzo {effort}, modelo {model}. Cambiar esfuerzo o modelo", nl: "Inspanning {effort}, model {model}. Inspanning of model wijzigen" },
  effMenu: { en: "Effort and model", es: "Esfuerzo y modelo", nl: "Inspanning en model" },
  effModel: { en: "Model", es: "Modelo", nl: "Model" },
  effNoModel: { en: "No model set", es: "Sin modelo", nl: "Geen model ingesteld" },
  effChange: { en: "Change", es: "Cambiar", nl: "Wijzigen" },
  effModels: { en: "Models", es: "Modelos", nl: "Modellen" },
  effModelNote: {
    en: "Sets the {role} model for every chat (also in Settings › Models).",
    es: "Cambia el modelo de {role} para todos los chats (también en Ajustes › Modelos).",
    nl: "Stelt het {role}-model in voor alle chats (ook in Instellingen › Modellen).",
  },
  effNoModels: { en: "No models found: add a provider in Settings.", es: "No hay modelos: añade un proveedor en Ajustes.", nl: "Geen modellen gevonden: voeg een provider toe in Instellingen." },
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
