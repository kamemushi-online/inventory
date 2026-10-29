"""One timestamped RSS observation of the explicitly selected viewer process."""
import datetime
import json
import subprocess
import sys

pid, checkpoint, output = sys.argv[1:]
raw = subprocess.check_output(["ps", "-p", str(int(pid)), "-o", "rss=,%cpu=,etime="], text=True)
rss, cpu, elapsed = raw.strip().split()
entry = {"timestamp": datetime.datetime.now().astimezone().isoformat(),
         "pid": int(pid), "checkpoint": checkpoint, "rss_mib": int(rss) / 1024,
         "cpu_percent": float(cpu), "process_elapsed": elapsed}
with open(output, "a") as stream:
    stream.write(json.dumps(entry) + "\n")
print(json.dumps(entry))
