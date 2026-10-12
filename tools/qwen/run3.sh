#!/bin/sh
# usage: trace3.sh <jobs.json> <treeprefix> ; three independent Coder runs in fresh trees, every call recorded
S=/tmp/claude-1000/-media-development/63e7bfbc-ab5d-426c-8a46-35181f730b00/scratchpad
SSH="ssh -i $HOME/.ssh/id_ed25519_kireserver kireserver"
JOBS=$1; P=$2; SRC=${3:-/media/development/kk-wt-stun2}
for r in 1 2 3; do
  rsync -a --delete -e "ssh -i $HOME/.ssh/id_ed25519_kireserver" --exclude node_modules --exclude target --exclude test-results --exclude .git $SRC/ kireserver:/tmp/$P-r$r/
  scp -q -i ~/.ssh/id_ed25519_kireserver $JOBS kireserver:/tmp/$P-r$r/jobs.json
  scp -q -i ~/.ssh/id_ed25519_kireserver $S/trace-driver.py kireserver:/tmp/trace-driver.py
  $SSH "cd /tmp/$P-r$r && rm -rf trace && mkdir -p docs/qwen-log && TRACE_OUT=/tmp/$P-r$r/trace KOMPANION_CONFIG=\$HOME/Docker/Personal-projects/kreative-kompanion/kompanion.toml timeout 2400 python3 /tmp/trace-driver.py /tmp/$P-r$r jobs.json docs/qwen-log/$P$r.jsonl > pipeline.out 2>&1; echo $P$r done"
done
