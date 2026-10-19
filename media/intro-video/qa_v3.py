#!/usr/bin/env python3
"""Decode a rendered movie and report technical continuity, not aesthetic quality."""
import argparse,json,subprocess
from pathlib import Path
import numpy as np
from PIL import Image,ImageDraw

def check(video,story_path,out):
    out.mkdir(parents=True,exist_ok=True)
    probe=json.loads(subprocess.check_output(['ffprobe','-v','error','-show_streams','-show_format','-of','json',str(video)]))
    story=json.loads(story_path.read_text());stream=next(s for s in probe['streams'] if s['codec_type']=='video')
    a,b=map(int,stream['r_frame_rate'].split('/'));fps=a/b
    boundaries=[s['start'] for s in story['scenes'][1:]]
    times={round(t*fps):t for boundary in boundaries for t in [boundary-1/fps,boundary,boundary+.4,boundary+.8]}
    times.update({0:0,round(story['duration']*.5*fps):story['duration']*.5,round(story['duration']*fps)-1:story['duration']-1/fps})
    w,h=384,216;size=w*h*3
    cmd=['ffmpeg','-v','error','-i',str(video),'-vf',f'scale={w}:{h}','-f','rawvideo','-pix_fmt','rgb24','-']
    p=subprocess.Popen(cmd,stdout=subprocess.PIPE);i=0;prev=None;deltas=[];black=[];frames={}
    while True:
        data=p.stdout.read(size)
        if not data:break
        if len(data)!=size:raise RuntimeError('Partial decoded frame')
        arr=np.frombuffer(data,np.uint8).reshape(h,w,3)
        if np.mean(arr)<3:black.append(i)
        if prev is not None:deltas.append((i,float(np.abs(arr.astype(float)-prev).mean())))
        if i in times:frames[i]=Image.fromarray(arr.copy())
        prev=arr.astype(float);i+=1
    if p.wait():raise RuntimeError('Video decoder failed')
    dm=dict(deltas)
    rows=[]
    for boundary in boundaries:
        n=round(boundary*fps)
        rows.append({'time':boundary,'delta_before':dm.get(n-1),'delta_at':dm.get(n),'delta_after':dm.get(n+1)})
    for page,start in enumerate(range(0,len(boundaries),5)):
        group=boundaries[start:start+5];sheet=Image.new('RGB',(w*4,(h+28)*len(group)),'#eeeeee');draw=ImageDraw.Draw(sheet)
        for row,t in enumerate(group):
            for col,stamp in enumerate([t-1/fps,t,t+.4,t+.8]):
                index=round(stamp*fps);sheet.paste(frames[index],(col*w,row*(h+28)));draw.text((col*w+8,row*(h+28)+h+6),f'{stamp:.3f}s',fill='black')
        sheet.save(out/f'transitions-{page+1:02}.jpg',quality=93)
    report={'scope':'Decoded-frame technical checks only; not continuous playback or aesthetic approval.','probe':probe,'decoded_frames':i,'black_frames':black,'boundary_deltas':rows,'largest_deltas':sorted(deltas,key=lambda v:v[1],reverse=True)[:20]}
    (out/'technical-qa.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
    for index in [0,round(story['duration']*.5*fps),round(story['duration']*fps)-1]:frames[index].save(out/f'decoded-{index:05}.png')
    print(json.dumps({'decoded_frames':i,'black_frames':len(black),'transition_sheets':page+1,'report':str(out/'technical-qa.json')}))
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--video',type=Path,required=True);p.add_argument('--out',type=Path,required=True);p.add_argument('--story',type=Path,default=Path(__file__).with_name('story-v3.json'));a=p.parse_args();check(a.video,a.story,a.out)
