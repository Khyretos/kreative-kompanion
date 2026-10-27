# Chat attachments (CHAT-08, CHAT-09)

Any file can be attached to a chat message. Nothing is refused for its type, and the only size limit is the one the user
sets (Settings > Attachments > "Largest attachment (MB)", 0 = no limit, the default). Uploads are streamed to disk, so a
multi-GB file does not fill memory.

## What is kept

| Kind | Stored in `chat-files/<chat>/` | What the model reads |
|---|---|---|
| `image` (PNG, JPG, WebP) | the picture | the picture, if the model sees pictures, else its name |
| `text` (text, code, PDF, web page, any unknown extension whose content is UTF-8 text) | `<uuid>.txt`: the text as it is (PDF and web page: the extracted text, PDF pages marked `=== page N ===`) | the text |
| `file` (everything else) | `<uuid>.bin` (the original, for download) and `<uuid>.txt` (its text view) | the text view |

The text view of a `file` (`attach_docs.rs`): Word (`.docx`) with headings, Excel (`.xlsx`) as tab-separated rows per sheet,
PowerPoint (`.pptx`) per slide with notes, OpenDocument (`.odt` `.ods` `.odp`), ePub, RTF, a listing of zip / tar / tar.gz
archives, the text of a `.gz` file, ffprobe's description of audio, video and pictures, and for any other binary the
readable strings in it. A file that cannot be read still gets a description, never an error.

## How a model of any size reads it

The sizes follow the model's context window (`context_window` of the provider in Settings, else a known size for cloud
models, else **8192 tokens**: an unknown model counts as a small one).

* **In the message:** the start of each file, up to a quarter of the window, shared by all the files of the prompt (the newest
  message first). A file that fits is shown whole. For the rest the message says how to read on.
* **`read_attachment`** (name, from_line, member): a part sized to an eighth of the window (1.2k to 48k characters). The first
  read of a file starts with a summary (lines, pages, headings or definitions, a CSV's columns); every read ends with the
  line to continue from. `member` reads a text file inside a zip or tar archive.
* **`search_attachment`** (query, name, context): lines containing the words, with line numbers and context; then
  `read_attachment from_line` near a match. This is the fast way into a big file for a small model.
* A line longer than 1000 characters counts as several lines, so line numbers work for minified files and one-line logs.
* A plain chat answer reads about a message's budget and a half per answer (then it says which line to go on from); the PC
  agent's older tool output is trimmed by its existing memory budget. The plain chat only runs the tool round when some
  file's text was not shown in full, because that round costs a model call.

## Known limits

* No OCR for pictures or scanned PDFs (a scan has no text layer), and no speech-to-text for audio: they are described, not read.
* 7z and RAR archives and the old binary Office formats (`.doc`, `.xls`, `.ppt`) are only given as their readable strings.
* Encrypted archives and documents are listed but not opened.
