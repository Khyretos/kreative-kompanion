# Lesson 201: the Coder filled the other stub too, and used `?` in a function returning bool

Situation: RUNNER-SYNC unit rsb1 (job in jobs-rsb1.json) asked for the body of `sha_ok` only; `install_binary` sat in the same file as a `todo!()` stub for the next unit. The Coder (OVMS, 3 attempts, check failed each time) wrote `sha_ok` with `sha_text.split_whitespace().next()?` (an `Option` question mark in a fn returning bool, compile error) and also replaced the `install_binary` stub with a 40-line body that no check had asked for.
Fix: The Coder also appended its own `mod test_install_binary` (lesson 193 family). Claude restored the `todo!()`, removed that module and wrote the `let Some(word) = ... else { return false };` line. wrong-output-selfupdate.rs.txt is the Coder's final file.
Rule: keep the next unit's stub out of the file (or in its own file) so the Coder has nothing to fill; say "return type is bool, no `?`" when the function returns bool. Same family as 183 and 196.
Rerun: jobs.json from this folder (rsb1.json) against the stub file with both `todo!()`.
