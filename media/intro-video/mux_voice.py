#!/usr/bin/env python3
"""Mix aligned narration with quiet original score; preserve video frames."""
import argparse
import json
import subprocess
from pathlib import Path

def duration(path):
    data=subprocess.check_output(['ffprobe','-v','error','-show_entries','format=duration','-of','json',str(path)])
    return float(json.loads(data)['format']['duration'])
def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--video',type=Path,required=True); p.add_argument('--voice',type=Path,required=True); p.add_argument('--out',type=Path,required=True)
    a=p.parse_args()
    if abs(duration(a.video)-duration(a.voice))>.05: raise ValueError('Audio/video timeline durations differ; align the narration first.')
    subprocess.run(['ffmpeg','-hide_banner','-loglevel','error','-y','-i',str(a.video),'-i',str(a.voice),
      '-filter_complex','[0:a]volume=0.18[m];[1:a]loudnorm=I=-18:TP=-1.5:LRA=11[v];[m][v]amix=inputs=2:duration=first:normalize=0,alimiter=limit=0.95:level=false[a]',
      '-map','0:v','-map','[a]','-c:v','copy','-c:a','aac','-b:a','192k','-ar','48000','-ac','2','-movflags','+faststart','-t',str(duration(a.video)),str(a.out)],check=True)
    print(a.out)
if __name__=='__main__':main()
