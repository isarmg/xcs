"""One fresh subprocess only: Linux ru_maxrss is KiB, builds excluded."""
import json, os, pathlib, resource, subprocess, sys, time
run=pathlib.Path(sys.argv[1]); revision=sys.argv[2]; count=int(sys.argv[3])
output=run/f'{revision}-{count}.json'
env=dict(os.environ, TMPDIR=str(run/'tmp'))
start=time.monotonic()
with output.open('w') as stdout, (run/f'{revision}-{count}.stderr').open('w') as stderr:
    try:
        proc=subprocess.run([str(run/f'target-{revision}'/'debug/zxs-perf-check'), str(count)],stdout=stdout,stderr=stderr,env=env,timeout=1200)
        status=proc.returncode
    except subprocess.TimeoutExpired:
        status='timeout-1200s'
usage=resource.getrusage(resource.RUSAGE_CHILDREN)
record={'revision':revision,'history_limit':count,'elapsed_seconds':round(time.monotonic()-start,6),'peak_rss_kib':usage.ru_maxrss,'user_seconds':usage.ru_utime,'system_seconds':usage.ru_stime,'exit_status':status,'profile':'dev (debug=0, incremental=false)','timings_are_gates':False,'rss_method':'Linux getrusage(RUSAGE_CHILDREN), one directly executed probe child, build excluded','pool_filesystem':'workspace overlay, not tmpfs','timeout_seconds':1200}
(run/f'{revision}-{count}-process.json').write_text(json.dumps(record,indent=2)+'\n')
print(json.dumps(record),flush=True)
if status != 0: sys.exit(1)
data=json.loads(output.read_text())
assert all(data['checks'].values())
