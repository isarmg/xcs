#!/usr/bin/env python3
"""Action-first mechanism sample. Deliberately not a product screen recording."""
import argparse
import json
import math
import subprocess
from pathlib import Path
from PIL import Image
from render import Canvas, ease, lerp, BLUE, INK, MUTED, GREEN, BORDER, LIGHT
HERE=Path(__file__).resolve().parent
DEFAULT=HERE/'story-v2.json'

def between(t,a,b): return ease((t-a)/(b-a))
def cursor(c,x,y,click=0):
    c.line([(x,y),(x+4,y+23),(x+10,y+15),(x+21,y+16),(x,y)],INK,3)
    if click>0: c.d.ellipse(((x-17*click)*c.s,(y-17*click)*c.s,(x+17*click)*c.s,(y+17*click)*c.s),outline=BLUE,width=2)
def pill(c,x,y,text,on=False,color=BLUE,width=130):
    c.box((x,y,x+width,y+42),color if on else 'white',None if on else BORDER,r=21)
    c.text(x+width/2,y+11,text,18,'white' if on else MUTED,True,anchor='mt')
def load_story(path):
    return json.loads(path.read_text())

def paper(c,x,y,value,branch,filename,width=760,highlight=False,editing=False,status='已保存'):
    c.box((x+2,y+8,x+width+2,y+408),'#e3e8f0',r=19)
    c.box((x,y,x+width,y+400),'white',BORDER,r=19)
    c.text(x+32,y+25,filename,27,INK,True)
    c.text(x+32,y+76,branch,18,BLUE,True)
    c.text(x+width-112,y+78,status,16,GREEN if status=='已保存' else MUTED)
    c.line([(x+30,y+116),(x+width-30,y+116)],BORDER,1)
    c.text(x+32,y+148,'宣传计划',19,MUTED)
    c.text(x+32,y+198,'投放渠道',24,MUTED)
    vx=x+32;vy=y+243
    if highlight:c.box((vx-5,vy-8,vx+len(value)*44+12,vy+52),LIGHT,r=6)
    c.text(vx,vy,value,44,BLUE,True)
    c.text(x+32,y+350,'内容编辑示意 · 改后文件通过上传保存' if editing else '文件内容预览',15,MUTED)

def frame(t,story,w=1920,caption_visible=True):
    c=Canvas(w,round(w*9/16));c.box((0,0,1280,720),'#f5f7fb',r=0)
    c.text(43,27,'ZXS',24,BLUE,True);c.text(118,32,story['disclosure'],16,MUTED)
    scenes=story['scenes'];idx=min(8,int(t/3));sc=scenes[idx];fn=story['file_name']
    before=scenes[0]['body_before'].split('：')[1];wrong=scenes[0]['body_after'].split('：')[1];changed=scenes[5]['body_after'].split('：')[1]
    labels=[(0,'改错以后，原来的内容还在吗？'),(3,'打开历史，选改动前的版本'),(6,'旧内容，实际打开看'),(9,'从这个旧版，另开一个方案'),(12,'进入方案 B，起点没有改变'),(15,'改好文件，上传到方案 B'),(18,'保存完成，出现 B 自己的版本'),(21,'切回主方案，检查内容'),(24,'再切到 B，检查两边都保留')]
    title=next(label for start,label in reversed(labels) if t>=start);c.text(55,91,title,30,INK,True)
    hp=between(t,3,3.6)*(1-between(t,12.4,13));x=lerp(260,65,hp);y=184;width=760 if hp<.1 else lerp(760,690,hp)
    value=before;branch='主方案';status='已保存';editing=t<2.1 or 15<=t<18
    if .65<=t<1.2:value=wrong[:max(1,int((t-.65)/.55*len(wrong)))]
    elif 1.2<=t<6:value=wrong
    elif 6<=t<12.8:value=before;branch='查看旧版 · 改动前'
    elif 12.8<=t<18:value=before;branch='方案 B · 沿用所选旧版'
    elif 18<=t<21.7:value=changed;branch='方案 B · 自有版本 1'
    elif 21.7<=t<24.7:value=wrong;branch='主方案'
    elif t>=24.7:value=changed;branch='方案 B · 自有版本 1'
    if .5<t<2.1:status='修改中' if t<1.5 else '上传中'
    if 16.8<t<18:status='上传中'
    if 15.6<t<16.8:value=before+changed[len(before):][:max(1,int((t-15.6)/1.1*len(changed[len(before):])))]
    pulse=any(a<=t<a+1.2 for a in [.5,6,12.8,15.6,18,21.7,24.7])
    if t<25.5:paper(c,x,y,value,branch,fn,width,pulse,editing,status)
    # Text selection and actual typed characters, with the edit context disclosed.
    if .5<t<1.2:cursor(c,x+153,y+266)
    if 1.5<t<2.1:
        pill(c,x+300,y+325,'上传当前方案',True,width=240);cursor(c,x+490,y+350,1)
    if 2.1<t<3:c.text(x+32,y+311,'改错了：公众号 → 短视频',21,'#b45309',True)
    # The historical versions enter only after the history action.
    if hp>.01:
        hx=lerp(1340,800,hp);c.box((hx,184,hx+385,584),'white',BORDER,r=19)
        c.text(hx+25,209,'文件历史',25,INK,True)
        for j,(label,val) in enumerate([('刚才',wrong),('改动前',before)]):
            yy=270+j*112;on=t>=4.9 and j==1
            c.box((hx+18,yy,hx+367,yy+92),LIGHT if on else '#f8fafc',BLUE if on else BORDER,r=10,width=2)
            c.text(hx+35,yy+15,label,17,MUTED)
            c.text(hx+35,yy+47,val,25,BLUE if on else INK,True)
        if 4.1<t<5.8:cursor(c,hx+255,lerp(322,427,between(t,4.1,4.9)),1 if 4.8<t<5.1 else 0)
        if 7.5<t<10:
            pill(c,hx+52,505,'从此版本创建分支',True,width=280);cursor(c,hx+280,530,1 if 9.1<t<9.5 else 0)
    if 6.5<t<7.5:c.text(x+33,y+314,'主方案是短视频；这里读到公众号',18,GREEN)
    # Naming a reference from the selected old version, without copying a document.
    if 9.8<=t<12.8:
        c.box((375,239,905,529),'#f2f5fa',BLUE,r=18)
        c.text(410,269,'创建新方案',25,INK,True);c.text(410,315,'起点：改动前 · 公众号',19,MUTED)
        c.box((410,355,870,415),'white',BORDER,r=8)
        name='方案B'[:min(3,max(0,int((t-10.8)/.3)+1))] if t>=10.8 else ''
        c.text(430,373,name,24,INK,True)
        pill(c,671,450,'创建',True,width=180)
        if t>11.8:cursor(c,815,475,1)
    if 12.8<=t<15:pill(c,455,603,'已切到方案 B · 正文仍是公众号',True,color=GREEN,width=470)
    # File upload is the write mechanism; no fictional in-browser editor claim.
    if 16.8<=t<18:
        pp=between(t,16.8,17.9);sx=lerp(30,x+430,pp);sy=lerp(330,y+160,pp)
        c.box((sx,sy,sx+300,sy+145),'white',BLUE,r=12,width=2)
        c.text(sx+18,sy+18,fn,19,INK,True);c.text(sx+18,sy+56,changed,25,BLUE,True)
        c.text(sx+18,sy+108,'改后文件 → 上传方案 B',16,MUTED);cursor(c,sx+250,sy+120)
    if 18<=t<20.5:pill(c,474,603,'方案 B 已保存 · 独立版本 1',True,color=GREEN,width=430)
    if 20.5<=t<25.5:
        ismain=21.7<=t<24.7
        pill(c,470,134,'主方案',ismain,width=160);pill(c,670,134,'方案 B',not ismain,width=160)
        if t<22:cursor(c,lerp(750,550,between(t,20.7,21.6)),157,1 if 21.5<t<21.8 else 0)
        if 24<t<25:cursor(c,lerp(550,750,between(t,24,24.6)),157,1 if 24.5<t<24.8 else 0)
    # Only after both have been opened do they sit side-by-side as evidence.
    if t>=25.5:
        move=between(t,25.5,26.3)
        paper(c,lerp(260,50,move),184,wrong,'主方案 · 刚刚已验证',fn,width=550,highlight=t<26.5)
        paper(c,lerp(1300,680,move),184,changed,'方案 B · 刚刚已验证',fn,width=550,highlight=t>=26.5)
    if caption_visible:c.text(640,661,sc['caption'],25,INK,anchor='mt')
    c.box((0,716,1280,720),'#e4e4e7',r=0);c.box((0,716,max(1,t/28*1280),720),BLUE,r=0)
    return c.im

def main():
    p=argparse.ArgumentParser();p.add_argument('--story',type=Path,default=DEFAULT);p.add_argument('--out',type=Path,required=True);p.add_argument('--width',type=int,default=1920);p.add_argument('--fps',type=int,default=30);p.add_argument('--frames-only',action='store_true')
    p.add_argument('--voice-report',type=Path);p.add_argument('--voice-id');p.add_argument('--audio',type=Path);p.add_argument('--lead',type=float,default=.6);p.add_argument('--tail',type=float,default=.8)
    a=p.parse_args();story=load_story(a.story);a.out.mkdir(parents=True,exist_ok=True)
    timing=None;duration=28
    if a.voice_report:
        report=json.loads(a.voice_report.read_text());timing=next(v for v in report['candidates'] if v['voice']==a.voice_id)
        duration=timing['timings'][-1]['end']+a.lead+a.tail
    def canonical(real):
        if not timing:return real
        real-=a.lead
        if real<0:return 0
        for i,seg in enumerate(timing['timings']):
            if real<seg['end']:
                sc=story['scenes'][i];q=max(0,min(1,(real-seg['start'])/(seg['end']-seg['start'])))
                return sc['start']+q*(sc['end']-sc['start'])
        return 27.999
    times=[.3,1.1,2.6,4.9,6.5,10.9,13.5,16.3,17.4,19,22.5,27]
    thumbs=[]
    for n,t in enumerate(times):
        frame(t,story,a.width).save(a.out/f'v2-{n+1:02d}-{t}.jpg',quality=95);thumbs.append(frame(t,story,640))
    sheet=Image.new('RGB',(1920,1440),'white')
    for n,im in enumerate(thumbs):sheet.paste(im,(n%3*640,n//3*360))
    sheet.save(a.out/'contact-sheet-v2.jpg',quality=95)
    if a.frames_only:return
    tag=a.voice_id or 'silent';dest=a.out/f'zxs-v2-action-sample-{tag}.mp4';w=a.width;h=round(w*9/16);frames=math.ceil(duration*a.fps);duration=frames/a.fps
    cmd=['ffmpeg','-hide_banner','-loglevel','error','-y','-f','rawvideo','-pix_fmt','rgb24','-s',f'{w}x{h}','-r',str(a.fps),'-i','-']
    if a.audio:cmd+=['-i',str(a.audio),'-af',f'adelay={round(a.lead*1000)},loudnorm=I=-18:TP=-1.5:LRA=11,apad','-c:a','aac','-b:a','160k','-ar','48000']
    cmd+=['-c:v','libx264','-preset','veryfast','-crf','19','-pix_fmt','yuv420p','-t',str(duration),'-movflags','+faststart',str(dest)]
    proc=subprocess.Popen(cmd,stdin=subprocess.PIPE)
    for i in range(frames):proc.stdin.write(frame(canonical(i/a.fps),story,w,not timing or i/a.fps>=a.lead).tobytes())
    proc.stdin.close()
    if proc.wait():raise RuntimeError('Encoding failed')
    print(dest)
if __name__=='__main__':main()
