# 253-system-rs-sample-split

system.rs. Asked to rename `sample` to `sample_unix` and add a `sample()` that calls it per OS, the Coder kept the name `sample` and only added `#[cfg(unix)]`, so Windows had no `sample` at all; the test job then called `sample_unix` that did not exist yet. Claude added the wrapper. Say `after this job there must be BOTH fn sample and fn sample_unix` and check with grep for both.
