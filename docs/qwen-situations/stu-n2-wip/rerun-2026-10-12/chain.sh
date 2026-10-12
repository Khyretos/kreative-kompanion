#!/bin/bash
set -o pipefail
S=/tmp/claude-1000/-media-development/88bde86a-eb4b-4688-a337-ec45384eec03/scratchpad
K="ssh -i $HOME/.ssh/id_ed25519_kireserver kireserver"
until grep -q "r3 done" $S/run-http.out; do sleep 5; done
for u in cast-http cast-mock cast-main-state cast-main-actions cast-main-input cast-main-make cast-css cast-rust-meta; do
  if [ $u != cast-http ]; then ( cd $S && ./unit.sh $u > run-$u.out 2>&1 ); fi
  $K "cd /tmp/kk-n2d && python3 accept.py $u n2-jobs-v2.json" | tee -a $S/chain.log || { echo "STOP at $u" >> $S/chain.log; exit 1; }
done
echo ALLDONE >> $S/chain.log
