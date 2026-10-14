# 262: Windows paths in grants (RUNNER-WIN-2 unit A)

platform.rs: `plain_path_str` cut the `\\?\UNC\` prefix one character too far (`&p[9..]`, the prefix is 8 characters). grants.rs: asked for three one-line replacements, the Coder added an extra `is_windows_path` rule that rejected every Windows target ending in a letter (`C:\Users\k`) and called a function that does not exist. Claude restored grants.rs and applied the three replacements by script. Rule: count prefix lengths with `strip_prefix` instead of slice numbers; a prompt for one-line replacements says `add no other rule`.
