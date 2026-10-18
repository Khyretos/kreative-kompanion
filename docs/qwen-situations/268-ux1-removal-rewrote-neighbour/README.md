# 268: a removal job rewrote a neighbouring line (STU-UX1 unit 4)

Job: remove the old progress bar line and the now unused `const running`. The Coder did both and also
changed the ring line Claude had corrected in unit 3 into an unconditional `${ringHtml(job.progress)}`, so
every finished card showed a ring at 100 %. The unit's check passed: it only looked at the running card.
The ring covered the picture, so unit 12's zoom check failed 3 times on a click that the ring intercepted.
Claude restored the condition and added the negative asserts (done and failed cards have no ring).
Files: `before-main.js`, `wrong-main.js`, `corrected-main.js`, `job.json`, `trace.tgz`.
