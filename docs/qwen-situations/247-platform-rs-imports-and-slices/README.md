# 247-platform-rs-imports-and-slices

platform.rs. Missing `use std::fs`, `h.as_str()` on an Option, `&p[1..]` kept the slash so `join` replaced the base path (must be `&p[2..]`), empty APPDATA/HOME not treated as unset although the tests say so, `unwrap_or_else(|| ..)` on a Result. The 3 fix rounds could not see the real errors: 100+ lines of unrelated warnings from the other crate filled the check output (see 252). Run checks with RUSTFLAGS="-A warnings" so the fix prompt holds only errors.
