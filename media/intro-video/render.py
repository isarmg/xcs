#!/usr/bin/env python3
"""XCS original explainer: Pillow scene graph -> ffmpeg rawvideo, no frame dump."""
import argparse, json, math, os, re, subprocess, wave
from pathlib import Path
from functools import lru_cache
import numpy as np
from PIL import Image, ImageDraw, ImageFont
HERE=Path(__file__).resolve().parent
ROOT=HERE.parent.parent
BG='#fafafa'; INK='#18181b'; MUTED='#71717a'; BLUE='#2563eb'; LIGHT='#eff6ff'; BORDER='#e4e4e7'; GREEN='#15803d'; ORANGE='#b45309'
FONT='/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc'
BOLD='/usr/share/fonts/opentype/noto/NotoSansCJK-Bold.ttc'
def ease(x):
    x=max(0,min(1,x)); return x*x*(3-2*x)
def lerp(a,b,p): return a+(b-a)*p
class Canvas:
    def __init__(self,w=1280,h=720):
        self.s=w/1280; self.im=Image.new('RGB',(w,h),BG); self.d=ImageDraw.Draw(self.im)
    def box(self,b,fill='white',outline=None,r=18,width=1):
        self.d.rounded_rectangle(tuple(round(x*self.s) for x in b),radius=round(r*self.s),fill=fill,outline=outline,width=max(1,round(width*self.s)))
    def line(self,xy,fill=BORDER,width=3): self.d.line([(round(x*self.s),round(y*self.s)) for x,y in xy],fill=fill,width=max(1,round(width*self.s)))
    def circle(self,x,y,r,fill=BLUE): self.d.ellipse(((x-r)*self.s,(y-r)*self.s,(x+r)*self.s,(y+r)*self.s),fill=fill)
    def text(self,x,y,text,size=24,color=INK,bold=False,anchor='lt'):
        tile=text_tile(str(text),round(size*self.s),color,bold)
        if anchor=='mt': x-=tile.width/self.s/2
        self.im.paste(tile,(round(x*self.s),round(y*self.s)),tile)
    def arrow(self,x1,y1,x2,y2,p=1,color=BLUE):
        x2,y2=lerp(x1,x2,ease(p)),lerp(y1,y2,ease(p)); self.line([(x1,y1),(x2,y2)],color,3)
        a=math.atan2(y2-y1,x2-x1)
        self.line([(x2-10*math.cos(a-.5),y2-10*math.sin(a-.5)),(x2,y2),(x2-10*math.cos(a+.5),y2-10*math.sin(a+.5))],color,3)
    def card(self,x,y,w,h,label,sub='',accent=BLUE):
        self.box((x,y+5,x+w,y+h+5),'#f0f0f2',r=19)
        self.box((x,y,x+w,y+h),'white',BORDER)
        self.box((x+16,y+17,x+22,y+h-17),accent,r=3)
        self.text(x+38,y+22,label,25,bold=True)
        if sub: self.text(x+38,y+62,sub,17,MUTED)
    def file(self,x,y,label,sub='',active=False):
        self.card(x,y,210,106,label,sub,BLUE if active else '#a1a1aa')
    def dotflow(self,a,b,p,color=BLUE):
        self.line([a,b],BORDER,3)
        for k in range(3):
            q=(p*.5+k/3)%1
            self.circle(lerp(a[0],b[0],q),lerp(a[1],b[1],q),5,color)
@lru_cache(maxsize=3000)
def text_tile(text,size,color,bold):
    font=ImageFont.truetype(BOLD if bold else FONT,size,index=2)
    box=font.getbbox(text); im=Image.new('RGBA',(max(1,box[2]-box[0]+4),max(1,box[3]-box[1]+4)))
    ImageDraw.Draw(im).text((2-box[0],2-box[1]),text,font=font,fill=color); return im

def wrap(text,width=42):
    # Chinese fullwidth characters take twice the width of Latin.
    lines=[]; line=''; n=0
    for ch in text:
        inc=1 if ord(ch)<128 else 2
        if n+inc>width: lines.append(line); line=''; n=0
        line+=ch; n+=inc
    if line: lines.append(line)
    return lines

def caption_parts(text):
    if isinstance(text,list): return text
    return [p.strip() for p in re.split(r'(?<=[。！？；])',text) if p.strip()]

def render(story,t,w=1280,h=720,screenshot=None):
    scenes=story['scenes']; sc=next((s for s in scenes if s['start']<=t<s['end']),scenes[-1]); i=scenes.index(sc)
    u=t-sc['start']; dur=sc['end']-sc['start']; p=u/dur; c=Canvas(w,h)
    c.text(48,25,'XCS',22,BLUE,True); c.text(108,29,'文件 · 版本 · 分层备份',14,MUTED)
    c.text(1232,29,f'{i+1:02d} / {len(scenes):02d}',14,MUTED,anchor='mt')
    title=sc['title']; c.text(64,76,title,38 if len(title)<=26 else 31,bold=True)
    c.text(66,133,sc.get('subtitle',''),19,MUTED)
    kind=sc['kind']; sid=sc['id']; q=ease(u/1.8); dy=(1-q)*25
    if kind=='pain':
        for j,(label,sub) in enumerate([('最终版','真的最终吗？'),('最终版_2','哪份改过？'),('最终版_新','哪份能恢复？')]):
            x=135+j*350; y=235+dy+math.sin(u*.7+j)*8
            c.file(x,y,label,sub,j==int(p*3)%3)
            c.text(x+105,y+137,['误覆盖','副本混乱','磁盘损坏'][j],22,ORANGE,anchor='mt')
        c.line([(242,430),(1038,430)],BORDER,2); c.text(640,453,'文件还在，不代表历史和备份都在',23,MUTED,anchor='mt')
    elif kind=='cause':
        for j,(label,sub) in enumerate([('历史版本','以前写过什么？'),('方案分支','不同方案如何并行？'),('物理副本','内容保护在哪里？')]):
            c.card(80+j*400,225+dy,320,150,label,sub,BLUE if p>j/3 else '#a1a1aa')
            c.text(240+j*400,405,['时间','关系','位置'][j],30,BLUE,True,anchor='mt')
    elif kind=='solution':
        c.card(470,235,340,160,'XCS 统一管理','一台 Linux 服务端',BLUE)
        labels=[('文件入口',80,260),('完整版本',960,210),('分层备份',960,360)]
        for label,x,y in labels:
            c.card(x,y,240,100,label,'')
        c.dotflow((320,310),(470,310),u); c.dotflow((810,280),(960,260),u); c.dotflow((810,350),(960,410),u); c.arrow(320,310,470,310,q); c.arrow(810,280,960,260,ease((u-1)/2)); c.arrow(810,350,960,410,ease((u-2)/2))
        c.text(640,453,'同一套核心，连接操作入口与存储规则',23,MUTED,anchor='mt')
    elif kind=='versions':
        phase=min(2,int(p*3)); phase_progress=(p*3)%1
        for j,label in enumerate(['工作区修改','封存完整内容','保护后发布']):
            x=170+j*355
            c.box((x,188,x+210,233),LIGHT if j<=phase else '#f4f4f5',r=10)
            c.text(x+105,200,label,20,BLUE if j<=phase else MUTED,True,anchor='mt')
            if j<2:
                c.dotflow((x+210,210),(x+355,210),u-j)
                c.arrow(x+210,210,x+355,210,ease((p*3-j)*2))
        c.line([(185,440),(1095,440)],BORDER,4)
        for j in range(3):
            x=170+j*355; active=p>j*.27
            c.file(x,280,f'完整版本 V{j+1}', ['初次发布','修改后发布','再次发布'][j],active)
            c.circle(x+105,440,12,BLUE if active else '#d4d4d8'); c.line([(x+105,386),(x+105,428)],BORDER,2)
            c.text(x+105,477,'FULL · 独立内容',18,BLUE if active else MUTED,anchor='mt')
        marker=185+910*ease(p); c.circle(marker,440,5,'#93c5fd')
    elif kind=='branch':
        c.file(90,284,'父文件 · V1','冻结父版本',True)
        c.file(530,220,'主分支 · V3','后续更新',True)
        write=sid=='s06'; c.file(960,340,'新分支', '首次写入 → FULL' if write else '引用父版本 V1',write)
        c.arrow(300,305,530,272,q); c.arrow(300,365,960,390,ease((u-.5)/2))
        c.text(640,420,'父版本引用保持冻结',19,MUTED,anchor='mt')
        if write:
            c.dotflow((680,340),(960,390),u); c.box((595,285,860,332),LIGHT); c.text(727,298,'写入自己的完整内容',19,BLUE,anchor='mt')
        else:
            c.box((520,450,820,490),LIGHT); c.text(670,461,'分支创建时不新增 FULL',18,BLUE,anchor='mt')
    elif sid=='s07':
        c.card(80,240,300,180,'目录分支','保存路径与文件映射')
        for j,label in enumerate(['设计稿','说明文档','素材文件']):
            y=203+j*105; c.card(570,y,240,82,label,'',BLUE)
            c.dotflow((380,330),(570,y+41),u-j); c.arrow(380,330,570,y+41,ease((u-j*.5)/2))
            c.card(980,y,210,82,f'文件分支 {j+1}',''); c.arrow(810,y+41,980,y+41,q)
        c.text(230,447,'不可变目录历史',20,BLUE,anchor='mt')
    elif sid=='s08' or kind=='workflow':
        c.file(80,215,'分支 A','候选正文 A',True); c.file(80,365,'分支 B','候选正文 B')
        c.card(505,265,300,150,'显式选择','正文结果 + 归属分支')
        c.arrow(290,268,505,310,q); c.arrow(290,418,505,370,q)
        c.file(970,285,'合并结果','保存到选定分支',True); c.dotflow((805,340),(970,340),u); c.arrow(805,340,970,340,ease((u-1)/2))
        c.text(650,451,'由使用者决定合并结果',21,BLUE,anchor='mt')
    elif kind=='backup':
        labels=[('P01','第一层 · 主盘'),('B01','第二层 · 来源 P01'),('C01','第三层 · 来源 B01')]
        for j,(label,sub) in enumerate(labels):
            x=90+j*430; c.card(x,237,250,170,label,sub,BLUE)
            c.box((x+38,343,x+212,375),LIGHT,r=6); c.text(x+125,350,'FULL 完整副本',16,BLUE,anchor='mt')
            if j<2: c.dotflow((x+250,315),(x+430,315),u-j*2); c.arrow(x+250,315,x+430,315,ease((u-j*2)/2))
        c.box((340,441,940,486),'#f4f4f5'); c.text(640,453,'业务内容副本：本例 3 份；元数据：独立的三成员机制',18,MUTED,anchor='mt')
        if p>.7:
            c.box((505,499,775,537),'#f0fdf4',r=10); c.text(640,510,'保护与发布条件全部满足 ✓',17,GREEN,anchor='mt')
    elif kind=='restore':
        c.card(90,245,280,160,'原始盘位','故障 / 需要恢复','#a1a1aa')
        c.line([(110,260),(350,390)],'#d4d4d8',3)
        c.card(500,245,280,160,'可用完整副本','按来源与版本定位',BLUE)
        c.card(920,245,280,160,'恢复目标','校验后恢复内容',GREEN)
        c.dotflow((780,325),(920,325),u,GREEN); c.arrow(780,325,920,325,q,GREEN)
        c.text(640,449,'离线恢复工具 + 完整性校验',24,BLUE,anchor='mt')
    elif kind=='web':
        if screenshot and Path(screenshot).exists():
            shot=loadshot(str(screenshot)); shot=shot.crop((0,0,shot.width, min(550,shot.height))); maxw,maxh=round(850*c.s),round(335*c.s); shot=shot.copy(); shot.thumbnail((maxw,maxh),Image.Resampling.LANCZOS)
            c.box((60,190,930,535),'white',BORDER); c.im.paste(shot,(round(70*c.s)+(maxw-shot.width)//2,round(200*c.s)))
            c.text(965,235,'真实 Web 界面',23,BLUE,True)
            c.text(965,290,'按磁盘浏览',20); c.text(965,337,'版本与文件操作',20); c.text(965,384,'共用业务核心',20)
            c.text(965,462,'界面录自项目本地实例',14,MUTED)
        else:
            c.box((90,210,1190,480),LIGHT); c.text(640,310,'真实界面素材尚未准备',28,BLUE,anchor='mt')
    else:
        c.text(640,224,'文件有历史，副本有去处',47,INK,True,anchor='mt')
        for j,label in enumerate(['独立完整版本','明确分支关系','按盘分层备份']):
            c.card(90+j*410,340,280,110,label,'',BLUE)
        c.text(640,474,'持续开发中 · 设备故障与原生交互仍需专项验收',18,MUTED,anchor='mt')
    # Three chronological steps. Active step advances with scene's explanation.
    steps=sc.get('steps',[])[:3]; active=min(2,int(p*3))
    for j,st in enumerate(steps):
        if isinstance(st,dict): st=st.get('text',st.get('title',''))
        x=65+j*405; on=j==active
        c.box((x,552,x+385,598),LIGHT if on else '#f4f4f5',r=10)
        c.circle(x+22,575,12,BLUE if on else '#a1a1aa'); c.text(x+22,566,str(j+1),14,'white',True,anchor='mt')
        c.text(x+44,565,st,17,BLUE if on else MUTED)
    caps=caption_parts(sc.get('caption','')); idx=min(len(caps)-1,int(p*len(caps))) if caps else 0
    caption=caps[idx] if caps else ''
    # Optional absolute word/phrase timings from the speech renderer. In pauses,
    # retain the previous caption rather than jumping ahead of the narration.
    timings=sc.get('caption_timings',[])
    if timings:
        current=timings[0]
        for segment in timings:
            if t>=segment['start']: current=segment
        caption=current['text']
    lines=wrap(caption,94)
    for j,line in enumerate(lines[:2]): c.text(640,626+j*32,line,23,INK,anchor='mt')
    c.box((0,715,1280,720),'#e4e4e7',r=0); c.box((0,715,max(1,1280*t/story['duration']),720),BLUE,r=0)
    # Brief white transitions, with no obscuring overlay for the actual screenshot.
    alpha=max(0,1-u/.35,1-(dur-u)/.3)
    if alpha>0: c.im=Image.blend(c.im,Image.new('RGB',c.im.size,BG),min(1,alpha))
    return c.im
@lru_cache(maxsize=2)
def loadshot(path): return Image.open(path).convert('RGB')
def music(path,duration,sr=44100):
    """Original low-volume ambient score; generated oscillators, no samples."""
    a=np.zeros(int(duration*sr),dtype=np.float32)
    chords=[(130.81,164.81,196),(110,130.81,164.81),(87.31,110,130.81),(98,123.47,146.83)]
    for start in range(0,int(duration),6):
        n=min(6*sr,len(a)-start*sr); t=np.arange(n)/sr; env=np.minimum(t/.7,1)*np.minimum((n/sr-t)/1.8,1)
        tone=sum(np.sin(2*np.pi*f*t)*.035+np.sin(2*np.pi*f*2*t)*.008 for f in chords[(start//6)%4])
        a[start*sr:start*sr+n]+=tone*env
    a*=np.minimum(np.arange(len(a))/sr/3,1)*np.minimum((len(a)-np.arange(len(a)))/sr/4,1)
    pcm=(np.clip(a,-1,1)*32767).astype('<i2')
    with wave.open(str(path),'wb') as f: f.setnchannels(1); f.setsampwidth(2); f.setframerate(sr); f.writeframes(pcm.tobytes())
def main():
    ap=argparse.ArgumentParser(); ap.add_argument('--story',type=Path,default=HERE/'story-timed.json'); ap.add_argument('--out',type=Path,required=True); ap.add_argument('--width',type=int,default=1920); ap.add_argument('--fps',type=int,default=30); ap.add_argument('--duration',type=float); ap.add_argument('--preview',action='store_true'); ap.add_argument('--screenshot',type=Path,default=ROOT/'docs/images/web-file-manager.png'); args=ap.parse_args()
    story=json.loads(args.story.read_text()); args.out.mkdir(parents=True,exist_ok=True); w=args.width; h=round(w*9/16)
    times=[(s['start']+s['end'])/2 for s in story['scenes']]
    for n,t in enumerate(times): render(story,t,w,h,args.screenshot).save(args.out/f'scene-{n+1:02d}.jpg',quality=94)
    thumbs=[render(story,t,640,360,args.screenshot) for t in times]; sheet=Image.new('RGB',(1920,360*math.ceil(len(thumbs)/3)),'white')
    for n,im in enumerate(thumbs): sheet.paste(im,(n%3*640,n//3*360))
    sheet.save(args.out/'contact-sheet.jpg',quality=95)
    if args.preview: return
    if any(s['kind']=='web' for s in story['scenes']) and not args.screenshot.exists(): raise SystemExit('Final render requires real screenshot; capture project web UI first.')
    duration=args.duration or story['duration']; wav=args.out/'original-score.wav'; music(wav,duration)
    dest=args.out/'xcs-explainer-zh.mp4'
    cmd=['ffmpeg','-hide_banner','-loglevel','error','-y','-f','rawvideo','-vcodec','rawvideo','-pix_fmt','rgb24','-s',f'{w}x{h}','-r',str(args.fps),'-i','-','-i',str(wav),'-c:v','libx264','-preset','veryfast','-crf','20','-pix_fmt','yuv420p','-c:a','aac','-b:a','160k','-shortest','-movflags','+faststart',str(dest)]
    proc=subprocess.Popen(cmd,stdin=subprocess.PIPE)
    try:
        for f in range(round(duration*args.fps)):
            proc.stdin.write(render(story,f/args.fps,w,h,args.screenshot).tobytes())
            if f%(args.fps*10)==0: print(f'{f/args.fps:.0f}/{duration:.0f} sec',flush=True)
    finally: proc.stdin.close()
    if proc.wait(): raise RuntimeError('ffmpeg failed')
    subprocess.run(['ffprobe','-v','error','-show_entries','format=duration,size:stream=codec_name,width,height,r_frame_rate','-of','json',str(dest)],check=True)
    print(dest)
if __name__=='__main__': main()
