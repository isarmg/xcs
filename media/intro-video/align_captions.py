#!/usr/bin/env python3
"""Attach actual TTS phrase timings to an approved scene timeline."""
import argparse
import json
from pathlib import Path

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--story',type=Path,required=True)
    p.add_argument('--report',type=Path,required=True)
    p.add_argument('--out',type=Path,required=True)
    a=p.parse_args(); story=json.loads(a.story.read_text()); report=json.loads(a.report.read_text())
    reports={s['id']:s for s in report['scenes']}
    for scene in story['scenes']:
        r=reports[scene['id']]
        if r['speech_seconds']>scene['end']-scene['start']-.7:
            raise ValueError(f"Narration does not fit {scene['id']}; revise the scene timeline first")
        segments=r['caption_timings_relative']
        if not segments: raise ValueError(f"Missing phrase timings for {scene['id']}")
        scene['caption_timings']=[dict(start=round(scene['start']+v['start'],3),end=round(scene['start']+v['end'],3),text=v['text']) for v in segments]
    a.out.write_text(json.dumps(story,ensure_ascii=False,indent=2)+'\n')
    print(a.out)
if __name__=='__main__': main()
