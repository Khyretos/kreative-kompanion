# Knowledge

Collections of documents a chat can search; answers cite "[collection / document, part n]".

## Add documents in the app

Capabilities page > Knowledge: type a name and Create; open Details; Add files (Markdown, text, HTML, PDF; PDF needs a text layer, scanned PDFs without text are refused); a progress bar per file; a refused file shows why; uploading a file with the same name again replaces it; the trash button removes a document; Delete collection removes all of it.

## Collections per project

Details > "Use in projects": tick projects. A chat in that project searches knowledge by itself, even without picking Knowledge in the chat's Tools menu, and its project's collections come first in the results. Other chats turn on Knowledge in the Tools menu.

## Search by words and by meaning

Every document is split into parts of about 1,500 characters. A question is searched by words (SQLite FTS5) and by meaning (vectors from the Embedder model in [assets] embed_url / embed_model, stored with sqlite-vec); both lists are merged (reciprocal rank fusion). New parts get vectors in the background, 32 at a time; the card shows "search by meaning N%" until all are done. Without an Embedder the word search still works.

## Open WebUI collections

One-off: `kompanion-server knowledge-import-openwebui <webui.db> [user name]`. Kept in step: mount Open WebUI's data folder read-only and set
```toml
[knowledge]
openwebui_db = "/openwebui/webui.db"
openwebui_user = "kees"   # empty: the first admin
```
with a compose volume like `- /path/to/open-webui-data:/openwebui:ro`. Every 10 minutes Kompanion checks whether webui.db (or webui.db-wal) changed, reads a copy, and adds new files, replaces changed ones and removes deleted ones. Those collections show "Open WebUI" on their card and take no uploads (add files in Open WebUI). A collection deleted in Open WebUI stays in Kompanion until you delete it there.

## API

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | /api/knowledge | List collections with counts, projects, the 50 newest documents |
| POST | /api/knowledge | Create a collection {name} |
| DELETE | /api/knowledge/{id} | Delete a collection and all its documents |
| PUT | /api/knowledge/{id}/projects | Update projects {projects: [ids]} |
| POST | /api/knowledge/{id}/docs?name=<file name> | Upload file (body, up to 25 MB) |
| DELETE | /api/knowledge/{id}/docs/{doc} | Delete a document |

Writes need the X-Kompanion: 1 header like every other write.
