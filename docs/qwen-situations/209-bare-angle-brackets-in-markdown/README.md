# Lesson 209: an unquoted `<version>` in a Markdown sentence

Situation: RUNNER-SYNC unit rsdoc1 (rsdoc1.json, soucouyant, attempt 1, check passed). The section was right but contained `"Update to <version>"` outside backticks, which MegaLinter's markdownlint reads as inline HTML (MD033). The check did not look for it. wrong-output.diff.gz.
Fix: Claude reworded that phrase.
Rule: tell the Coder "put anything with angle brackets in backticks" for Markdown units, and make the check grep for `<[a-z]` outside backticks.
