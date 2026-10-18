# 293: cfg inverted call (WIN-RESTART)

WIN-RESTART unit C (main.rs, OVMS, 3 attempts): put the call before the download (twice, once without cfg) and the one before `exit(4)` under `#[cfg(not(windows))]`, the opposite of the job; the comment was not updated. Claude fixed 3 lines. A repair round that sees "must be under cfg(windows)" added code instead of changing the attribute.

Inputs: the tree before the unit is Kompanion main 252b42a plus the earlier WIN-RESTART units; `job.json` is the exact job, `wrong-*` the Coder's last attempt, `corrected-*` the file Claude kept. Traces: kireserver /tmp/kk-wr/trace/winrestart-c-call/.
