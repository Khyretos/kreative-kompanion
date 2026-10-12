# Lesson 204: install_binary drafted with a wrong API and no cleanup on early returns

Situation: RUNNER-SYNC unit rsb2 (rsb2.json; tests in runner/src/selfupdate_tests.rs): write the body of `install_binary` (write `<target>.new`, chmod 755, run `--version`, rename). The Coder (soucouyant Ollama qwen3.5, 3 attempts, check failed) used `std::os::unix::fs::set_permissions` (does not exist; it is `std::fs::set_permissions`), left an unused `OnceLock` and a cleanup closure that was only called on one of four error paths, and `if let Err(_) = ... {}` instead of `let _ =`. wrong-output-selfupdate.rs.txt.gz is its final file; the fixed body is the one in runner/src/selfupdate.rs.
Fix: Claude rewrote the body with an inner closure doing all steps and one cleanup after it.
Rule: when a prompt lists the steps and says "cleanup in one place", also give the exact std paths (`std::fs::set_permissions`, `std::fs::Permissions::from_mode`) and name the failure paths the check covers.
