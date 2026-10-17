# 264: shared path rules (RUNNER-WIN-2 unit D)

paths.rs (machine-stats): the Coder (3 attempts) wrote `plain_path_str` with slice numbers and a `replace('\\', "/")` of the UNC rest, left out the backslash normalisation in `starts_with_target` and moved a String it used afterwards (E0382). The five server one-line edits were fine but failed their checks only because paths.rs did not compile (a failed unit poisons the following checks, lesson 252). Claude wrote the four path functions; the Coder's server edits stay.
