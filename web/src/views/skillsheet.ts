import { modal } from "../core/modal";
import { renderMarkdown } from "../core/markdown";
import type { KompanionApi } from "../api/client";

export interface SkillRef { id: string; layer: string; file: string }

/** SK-03: one skill card in a sheet: its text, edit (saved as a commit in the layer's repo), its git history, and per lesson a move between general and private. */
export async function openSkillSheet(
  api: KompanionApi,
  ref: SkillRef,
  opener: HTMLElement,
  onError: (e: unknown) => void,
  onSaved: () => void
): Promise<void> {
  const box = document.createElement("div");
  box.className = "skill-sheet";

  const sheet = document.createElement("div");
  sheet.className = "sheet";
  sheet.setAttribute("role", "dialog");
  sheet.setAttribute("aria-modal", "true");
  sheet.setAttribute("aria-labelledby", "skill-title");

  // Close button
  const closeBtn = document.createElement("button");
  closeBtn.className = "icon-btn sheet-close";
  closeBtn.setAttribute("aria-label", "Close");
  closeBtn.textContent = "×";
  closeBtn.addEventListener("click", () => {
    if (isDirty && !confirm("Discard your edits?")) return;
    m.requestClose();
  });

  // Header
  const title = document.createElement("h2");
  title.id = "skill-title";
  title.textContent = `Skill: ${ref.id}`;

  const layerChip = document.createElement("span");
  layerChip.className = "chip role";
  layerChip.textContent = ref.layer;

  title.append(" ", layerChip);

  const meta = document.createElement("p");
  meta.className = "muted small";
  meta.textContent = `${ref.file} in the ${ref.layer} layer`;

  // Body container
  const body = document.createElement("div");
  body.className = "skill-body";

  // Actions container
  const actions = document.createElement("div");
  actions.className = "skill-actions";

  const editBtn = document.createElement("button");
  editBtn.className = "btn";
  editBtn.textContent = "Edit";
  editBtn.addEventListener("click", () => switchToEdit());

  const historyBtn = document.createElement("button");
  historyBtn.className = "btn";
  historyBtn.textContent = "History";
  historyBtn.addEventListener("click", () => showHistory());

  actions.appendChild(editBtn);
  actions.appendChild(historyBtn);

  // Moving a lesson between general and private (the Kompanion layer has no counterpart).
  const movable = ref.layer === "general" || ref.layer === "private";
  const lessonsSection = document.createElement("section");
  lessonsSection.hidden = !movable;

  const lessonsTitle = document.createElement("h3");
  lessonsTitle.textContent = "Lessons";

  const lessonsList = document.createElement("ul");
  lessonsList.className = "skill-lessons";

  lessonsSection.append(lessonsTitle, lessonsList);

  const historyList = document.createElement("ol");
  historyList.className = "skill-history";
  historyList.hidden = true;

  const editor = document.createElement("div");
  editor.className = "skill-editor";

  sheet.appendChild(closeBtn);
  sheet.appendChild(title);
  sheet.appendChild(meta);
  sheet.appendChild(body);
  sheet.appendChild(actions);
  sheet.appendChild(historyList);
  sheet.appendChild(lessonsSection);
  box.appendChild(sheet);
  document.body.appendChild(box);

  let currentText = "";
  let isDirty = false;

  async function loadSkill() {
    try {
      const data = await api.getSkill(ref.id, ref.layer);
      currentText = data.text ?? "";
      renderBody(currentText);
    } catch (e) {
      onError(e);
    }
  }

  function renderBody(text: string) {
    body.replaceChildren(renderMarkdown(text));
  }

  /** Back from the editor to the card view. */
  function closeEditor() {
    editor.remove();
    editor.replaceChildren();
    isDirty = false;
    body.hidden = false;
    actions.hidden = false;
    lessonsSection.hidden = !movable;
  }

  function switchToEdit() {
    const textarea = document.createElement("textarea");
    textarea.className = "skill-edit";
    textarea.rows = 20;
    textarea.value = currentText;
    textarea.setAttribute("aria-label", "Card text");
    textarea.addEventListener("input", () => {
      isDirty = true;
    });

    const messageInput = document.createElement("input");
    messageInput.type = "text";
    messageInput.className = "skill-message";
    messageInput.placeholder = "What changed (commit message)";
    messageInput.setAttribute("aria-label", "Commit message");

    const saveBtn = document.createElement("button");
    saveBtn.className = "btn primary";
    saveBtn.textContent = "Save";

    const cancelBtn = document.createElement("button");
    cancelBtn.className = "btn";
    cancelBtn.textContent = "Cancel";
    cancelBtn.addEventListener("click", () => {
      if (isDirty && !confirm("Discard your edits?")) return;
      closeEditor();
    });

    saveBtn.addEventListener("click", async () => {
      saveBtn.disabled = true;
      try {
        const commit = await api.saveSkill(ref.layer, ref.file, textarea.value, messageInput.value);
        currentText = textarea.value;
        renderBody(currentText);
        closeEditor();
        const savedMeta = document.createElement("p");
        savedMeta.className = "muted small";
        savedMeta.textContent = `Saved as commit ${commit.commit.slice(0, 7)}.`;
        meta.parentNode?.insertBefore(savedMeta, meta.nextSibling);
        onSaved();
        buildLessonList();
      } catch (e) {
        onError(e);
        saveBtn.disabled = false;
      }
    });

    editor.replaceChildren(textarea, messageInput, saveBtn, cancelBtn);
    body.hidden = true;
    actions.hidden = true;
    lessonsSection.hidden = true;
    body.after(editor);
    textarea.focus();
  }

  async function showHistory() {
    try {
      const historyData = await api.skillHistory(ref.layer, ref.file);
      historyList.hidden = false;
      historyList.replaceChildren();

      if (historyData.commits.length === 0) {
        const li = document.createElement("li");
        li.textContent = "No commits yet.";
        historyList.appendChild(li);
      } else {
        for (const commit of historyData.commits) {
          const li = document.createElement("li");
          const code = document.createElement("code");
          code.textContent = commit.sha.slice(0, 7);
          const text = document.createTextNode(` ${commit.message} (${commit.author}, ${commit.date.slice(0, 10)})`);
          li.appendChild(code);
          li.appendChild(text);
          historyList.appendChild(li);
        }
      }
    } catch (e) {
      onError(e);
    }
  }

  function buildLessonList() {
    lessonsList.replaceChildren();
    if (!movable) return;
    const lines = currentText.split("\n");
    for (const line of lines) {
      const trimmed = line.trimStart();
      // Lessons start at the left edge; indented lines continue the one above.
      if (trimmed === line && /^(\d+\.|-) /.test(line)) {
        const li = document.createElement("li");
        const span = document.createElement("span");
        span.textContent = line;
        li.appendChild(span);

        const other = ref.layer === "general" ? "private" : "general";
        const btn = document.createElement("button");
        btn.className = "btn small";
        btn.textContent = `Move to ${other}`;
        btn.addEventListener("click", async () => {
          btn.disabled = true;
          try {
            await api.moveLesson(ref.layer, other, ref.file, line);
            currentText = takeLesson(currentText, line);
            renderBody(currentText);
            buildLessonList();
            onSaved();
          } catch (e) {
            onError(e);
            btn.disabled = false;
          }
        });
        li.appendChild(btn);
        lessonsList.appendChild(li);
      }
    }
  }

  /** The text without the lesson starting at `first` and its indented lines (the server's take_lesson). */
  function takeLesson(text: string, first: string): string {
    const lines = text.split("\n");
    const start = lines.findIndex((l) => l.trim() === first.trim());
    if (start < 0) return text;
    let end = start + 1;
    while (end < lines.length && /^[ \t]/.test(lines[end]) && lines[end].trim() !== "") end++;
    lines.splice(start, end - start);
    return lines.join("\n");
  }

  await loadSkill();
  buildLessonList();

  const m = modal(box, () => {
    box.remove();
    document.removeEventListener("keydown", esc);
  });

  const esc = (ev: KeyboardEvent) => {
    if (ev.key === "Escape" && (!isDirty || confirm("Discard your edits?"))) m.requestClose();
  };

  document.addEventListener("keydown", esc);
  m.open(opener);
}
