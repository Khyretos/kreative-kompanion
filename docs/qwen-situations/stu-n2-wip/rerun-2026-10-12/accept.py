# usage: accept.py unit : copy the out file of the first passing run into progress; exit 1 if none passed
import json,subprocess,sys
u=sys.argv[1]; jobs=json.load(open(sys.argv[2]))
out=[j for j in jobs if j["name"]==u][0]["out"]
for r in (1,2,3):
    log=f"/tmp/kk-n2d/{u}-r{r}/docs/qwen-log/{u}-r{r}.jsonl"
    last=[json.loads(l) for l in open(log)][-1]
    print(u,"r%d"%r,"passed" if last.get("check_passed") else "FAILED",last.get("check_attempts"))
    if last.get("check_passed") and "ok" not in globals():
        subprocess.check_call(["cp",f"/tmp/kk-n2d/{u}-r{r}/{out}",f"/tmp/kk-n2d/progress/{out}"]); ok=r
        print("accepted r%d"%r)
sys.exit(0 if "ok" in globals() else 1)
