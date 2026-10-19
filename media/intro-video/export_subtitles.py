#!/usr/bin/env python3
"""Export editable SRT from the final absolute narration phrase timings."""
import argparse
import json
from pathlib import Path

def timestamp(seconds):
    ms=round(seconds*1000); h,ms=divmod(ms,3600000); m,ms=divmod(ms,60000); s,ms=divmod(ms,1000)
    return f'{h:02}:{m:02}:{s:02},{ms:03}'
def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--story',type=Path,default=Path(__file__).with_name('story-timed.json'))
    p.add_argument('--out',type=Path,default=Path(__file__).with_name('captions.zh-CN.srt'))
    a=p.parse_args();story=json.loads(a.story.read_text());blocks=[]
    for scene in story['scenes']:
        for v in scene['caption_timings']:
            blocks.append(f"{len(blocks)+1}\n{timestamp(v['start'])} --> {timestamp(v['end'])}\n{v['text']}\n")
    a.out.write_text('\n'.join(blocks),encoding='utf-8');print(a.out)
if __name__=='__main__':main()
