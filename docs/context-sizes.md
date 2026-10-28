# Context sizes (CTX-02)

A model's context size, in tokens, decides how much of a chat Kompanion keeps before it summarises, what the context meter
shows, and how much of an attached file is shown to the model and read in parts (see `chat-attachments.md`). A model whose
size Kompanion does not know counts as a small one (8192 tokens), so a small model is never overloaded.

## Where the size comes from

Per provider and model, the first of these that exists:

1. **Set by hand**: Settings > Connected models > the model's "Context size (tokens)" (admins). Empty goes back to automatic.
2. **Learned from a refusal**: when a server refuses a prompt as too long and names its limit, Kompanion keeps it for that model
   (only ever lowered), tells you in the error, and keeps prompts within it from then on. Detect forgets it.
3. **The provider's own `context_window`** (`kompanion.toml` or the provider form).
4. **Detected**: asked from the server the first time the model is used, again every 6 hours in the background, and with the
   Detect button. Only reads, never a chat request, never anything that loads a model:

   | Server | Asked | Gives |
   |---|---|---|
   | vLLM, OpenRouter and others | `GET /models` | `max_model_len`, `context_length`, `context_window`, `max_context_length` |
   | llama.cpp | `GET /props` | `n_ctx` the server started with |
   | Ollama | `GET /api/ps`, else `POST /api/show` | the loaded context, else `num_ctx`, else Ollama's default 4096 (not the model's maximum) |
   | LM Studio | `GET /api/v0/models` | `loaded_context_length` of a loaded model |
   | Anthropic | `GET /v1/models/{id}` | `max_input_tokens` |
5. **Known by name**: common cloud models (Claude, GPT, Gemini, DeepSeek, Grok, Mistral).

Refusal messages that are understood: OpenAI, vLLM, DeepSeek ("maximum context length is N"), Mistral, Anthropic
("> N maximum"), Gemini, llama.cpp (`n_ctx`), and ovms-queue's guard ("about M tokens, the limit is L", scaled by how the prompt
measured against it, with a 15% margin).

## A server that says nothing: OVMS and its queue

OVMS lists models without sizes and has no endpoint for it. On kireserver the size of Coder is set by hand. The model supports
262,144 tokens; the limit is the A770's memory. `ovms-queue` (`size-guard.lua`) refuses a request over 98,000 *estimated*
tokens (HTTP 413), and its estimate runs about 1.6 times the real count for ordinary text.
Measured 2026-10-28 through the queue: 150 KB of documentation = 45,794 real tokens, answered correctly (a fact hidden in the
middle was found); 230 KB was refused as "about 110829 tokens". So about 62,000 real tokens of ordinary text fit, up to about
93,000 of number-heavy text. `context_window = 60000` in `kompanion.toml` keeps a margin for any text.

To measure a server of your own: send one request of known size through the same path Kompanion uses and read `usage.prompt_tokens`
from the answer; then one larger request, and note where it is refused or fails.
