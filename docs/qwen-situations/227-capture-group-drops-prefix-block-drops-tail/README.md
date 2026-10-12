# Lesson 227: netlib.py helpers against a given test file (2026-10-12, admin-lock/NetBird thread)

- Job: write netlib.py so test_netlib.py passes (job.json).
- Coder (OVMS): 1 draft + 2 repair rounds; 2 of 7 tests still failed:
  1. `uses_kk` returned names without the `KK_` prefix: the capture group started after `KK_`
     although the test expects `{"KK_LAN_IP", ...}`.
  2. `render_block` removed stray definitions only for the literal key `KK_=` and only in the
     replace branch, and silently dropped everything after the END marker.
- Claude corrected both (corrected-output.py).

Rules: (a) when a test compares exact strings or sets, copy the expected value's
shape into the capture group (include the prefix); (b) "remove lines that define a key in
values" means comparing each line's key with `values`, in every branch, and replacing a
block must keep the text after it.

Raw files (job.json, inputs, wrong and corrected outputs, check.sh) stay on kireserver in `~/Docker/docs/qwen-situations-pending/netlib-render-block-uses-kk/`: they hold network and firewall configuration, so they are kept out of this repository.
