# 249-selfupdate-helpers-ignored-spec

selfupdate.rs. Asked for helpers that work on every OS (so the tests run on Linux) the Coder wrapped the whole rename logic in `#[cfg(windows)]` with a `#[cfg(not(windows))]` stub, used `file_stem()` (drops `.exe`) for the `.old` name, made `cleanup_old` a no-op on unix, and a fix round deleted `make_executable`. The install_binary edits (3 one-line replacements) were never applied in 3 attempts. Say `no cfg inside this function` when the logic must be testable on Linux; give each one-line replacement as exact old/new text.
