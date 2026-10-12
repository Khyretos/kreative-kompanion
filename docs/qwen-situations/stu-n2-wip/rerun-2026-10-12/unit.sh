#!/bin/bash
# usage: unit.sh <unit-name> [jobsfile]  : 3 independent Coder runs of one unit, each from the cumulative accepted tree, every call recorded
S=/tmp/claude-1000/-media-development/88bde86a-eb4b-4688-a337-ec45384eec03/scratchpad
KEY=$HOME/.ssh/id_ed25519_kireserver
SSH="ssh -i $KEY kireserver"
U=$1; JOBS=${2:-$S/n2-jobs.json}
$SSH 'test -d /tmp/kk-n2d/progress' || { $SSH 'mkdir -p /tmp/kk-n2d'; rsync -a -e "ssh -i $KEY" $S/n2base/ kireserver:/tmp/kk-n2d/progress/; }
scp -q -i $KEY $S/../../../../../../media/development/kk-wt-stun2/tools/qwen/record_run.py kireserver:/tmp/kk-n2d/trace-driver.py 2>/dev/null || scp -q -i $KEY /media/development/kk-wt-stun2/tools/qwen/record_run.py kireserver:/tmp/kk-n2d/trace-driver.py
python3 - "$U" "$JOBS" > $S/job-$U.json <<'PY'
import json,sys
u,f=sys.argv[1:3]
print(json.dumps([j for j in json.load(open(f)) if j["name"]==u]))
PY
for r in 1 2 3; do
  T=/tmp/kk-n2d/$U-r$r
  $SSH "rm -rf $T && mkdir -p /tmp/kk-n2d && cp -a /tmp/kk-n2d/progress $T && ln -sfn \$HOME/Docker/Personal-projects/kreative-kompanion/web/node_modules $T/web/node_modules && mkdir -p $T/docs/qwen-log"
  sed -e "s#/tmp/kk-n2c#$T#g" -e "s#kk-n2-target#kk-n2d-$U-r$r#g" $S/job-$U.json | $SSH "cat > $T/jobs.json"
  $SSH "cd $T && rm -rf trace && TRACE_OUT=$T/trace KOMPANION_CONFIG=\$HOME/Docker/Personal-projects/kreative-kompanion/kompanion.toml timeout 3000 python3 /tmp/kk-n2d/trace-driver.py $T jobs.json docs/qwen-log/$U-r$r.jsonl > pipeline.out 2>&1; echo $U r$r done; tail -c 600 pipeline.out"
done
