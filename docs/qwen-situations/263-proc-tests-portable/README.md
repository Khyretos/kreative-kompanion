# 263: portable process tests (RUNNER-WIN-2 unit B)

Six one-line test edits. The Coder (first try each) did them, but (1) wrote the CRLF replacement as `"\\r\\n"` (backslash text, not CR LF) so it would do nothing on Windows, and (2) the `test_cwd` job deleted the whole test instead of changing one line; my check counted `#[test]` and still passed because the file had one more test than I assumed. Claude fixed the string and restored `test_cwd` with the shell helper. Rule: a check for an edit inside a test names the test (`grep -c 'fn test_cwd'`), not a count of all tests.
