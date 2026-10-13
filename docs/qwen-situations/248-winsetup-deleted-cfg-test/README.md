# 248-winsetup-deleted-cfg-test

winsetup.rs. The patch (SEARCH the two lines `#[cfg(test)]` / `mod tests {`) lost the `#[cfg(test)]` line, `char::encode_utf16()` needs a buffer (use `str::encode_utf16()` and `to_le_bytes`), a `vec!["/Create", ..]` of &str where Vec<String> was asked, and an inner `#![allow(dead_code)]` after items. When a patch re-emits an anchor, name BOTH lines in REPLACE and check `grep -c '#\[cfg(test)\]'` in the check.
