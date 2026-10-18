# 292: detached restart spawn (WIN-RESTART)

WIN-RESTART unit B (setup.rs restart_task_later, OVMS, 3 attempts): instead of adding one function it rewrote install_service to `.spawn()` (so `.status`/`.stderr` no longer existed: 3 compile errors), imported `io::Stdio`, and invented a `schtasks /Change /TN "Kompanion Runner" /RL HIGHESTPRI` call with the wrong task name; restart_line() was never used. Claude wrote the 17-line function. Rule for prompts: say "add a NEW function, do not touch install_service" and list the exact calls.

Inputs: the tree before the unit is Kompanion main 252b42a plus the earlier WIN-RESTART units; `job.json` is the exact job, `wrong-*` the Coder's last attempt, `corrected-*` the file Claude kept. Traces: kireserver /tmp/kk-wr/trace/winrestart-b-spawn/.
