#!/bin/sh
# three independent Coder runs of the STU-N2 units, each in a fresh copy of the clean base on kireserver
S=/tmp/claude-1000/-media-development/63e7bfbc-ab5d-426c-8a46-35181f730b00/scratchpad
K="ssh -i $HOME/.ssh/id_ed25519_kireserver kireserver"
for r in 1 2 3; do
  rsync -a --delete -e "ssh -i $HOME/.ssh/id_ed25519_kireserver" --exclude node_modules --exclude docs/qwen-log $S/n2c/ kireserver:/tmp/kk-n2c-r$r/
  scp -q -i ~/.ssh/id_ed25519_kireserver $S/n2c-jobs.json kireserver:/tmp/kk-n2c-r$r/jobs.json
  $K "cd /tmp/kk-n2c-r$r && ln -sfn /tmp/kk-n2c/web/node_modules web/node_modules && mkdir -p docs/qwen-log && sed -i 's#/tmp/kk-n2c#/tmp/kk-n2c-r$r#' jobs.json && KOMPANION_CONFIG=\$HOME/Docker/Personal-projects/kreative-kompanion/kompanion.toml timeout 3000 python3 tools/qwen/pipeline.py jobs.json docs/qwen-log/run$r.jsonl > pipeline.out 2>&1; echo run$r done"
done
