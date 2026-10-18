# 267: progress ring put inside the class attribute (STU-UX1 unit 3)

Job: wrap the card media in `div.job-thumb`, add `is-loading`, and add `ringHtml(job.progress)` after
`${media}` for running jobs. In all 3 attempts the Coder put the `ringHtml(...)` expression inside the class
string (`class="job-thumb ... <svg ...>"`), and one repair round rewrote the finished `ringHtml` helper from
unit 1 (dropped `pathLength`, added a `stroke-dashoffset` style) although the job never named it.
Claude moved the expression after `${media}` and restored the helper. Files: `before-main.js`,
`wrong-main.js`, `corrected-main.js`, `job.json`, `trace.tgz`. Rerun: `stu-ux1-unit-checks`, unit 3.
