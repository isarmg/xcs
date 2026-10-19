#!/usr/bin/env python3
"""Ten-and-a-half-second branch-only animation; no audio or external image inputs."""
import argparse,json,subprocess,math
from pathlib import Path
from PIL import Image,ImageDraw
from render import Canvas,BLUE,INK,ease,lerp
from render_v3_animation import doc,mix,path,tween
HERE=Path(__file__).resolve().parent
DURATION=10.5
OLD=(120,180,300);MAIN=(820,120,340);NEW=(480,380,250)
FINAL_MAIN=(160,145,380);FINAL_NEW=(740,145,380)

def title(c,rect,text,alpha=1):
    if alpha<=0:return
    color=tuple(round(lerp(a,b,alpha)) for a,b in zip((237,242,248),(37,99,235)))
    c.text(rect[0]+rect[2]/2,rect[1]-52,text,29,color,True,anchor='mt')

def frame(t,w=1920):
    c=Canvas(w,round(w*9/16));c.box((0,0,1280,720),'#edf2f8',r=0)
    # The existing original remains visible throughout. The old page is a source,
    # not an additional body materialized by branch creation.
    separate=tween(t,6.0,8.4)
    main=mix(MAIN,FINAL_MAIN,separate)
    arc=math.sin(math.pi*separate)
    main=(main[0],main[1]-60*arc,main[2]-170*arc)
    old=mix(OLD,(-400,180,300),separate)
    # The reference grows out of the old page before any new body exists.
    link=tween(t,.65,2.2)*(1-separate)
    path(c,[(420,360),(550,360),(550,570),(605,570)],link)
    doc(c,8,old);title(c,old,'旧版')
    if t<2.3:
        focus=tween(t,0,.5)*(1-tween(t,1.7,2.3))
        c.box((old[0]-5,old[1]-5,old[0]+old[2]+5,old[1]+old[2]*570/480+5),None,BLUE,r=2,width=max(1,round(2*focus)))
    doc(c,12,main);title(c,main,'主方案')
    if t<4.25:
        if link>.99:
            c.d.ellipse((585*c.s,550*c.s,625*c.s,590*c.s),outline=BLUE,width=max(1,round(3*c.s)))
            c.text(605,655,'新方案',29,BLUE,True,anchor='mt')
        if t>=2.55:
            # A short, clearly temporary edit. The old 8 body is untouched.
            number='8' if t<3.25 else '6'
            c.text(415,604,'预算',25,INK)
            c.text(505,580,number,61,BLUE,True)
            c.text(575,608,'万元',25,INK)
            if t<3.25:c.line([(502,620),(553,620)],'#b45309',3)
            else:c.line([(559,588),(559,638)],BLUE,2)
    else:
        birth=tween(t,4.25,5.75)
        new=mix((600,565,10),NEW,birth)
        new=mix(new,FINAL_NEW,separate)
        new=(new[0],new[1]+65*arc,new[2])
        doc(c,6,new)
        # Avoid labeling a page while it passes in front of the main body.
        if t>=7.8:title(c,new,'新方案',tween(t,7.8,8.4))
    if t>=8.4:
        # A single concise budget comparison; no repeated zoom or extra ending.
        for rect,start in [(main,8.4),(new,8.8)]:
            x,y,pw=rect;progress=tween(t,start,start+.4)
            c.line([(x+pw*.065,y+pw*.85),(x+pw*.065+pw*.49*progress,y+pw*.85)],BLUE,3)
    return c.im

def main():
    p=argparse.ArgumentParser();p.add_argument('--out',type=Path,required=True);p.add_argument('--width',type=int,default=1920);p.add_argument('--render',action='store_true');a=p.parse_args();a.out.mkdir(parents=True,exist_ok=True)
    if not a.render:
        times=[.5,2.25,3.6,5.7,7.2,9.6]
        sheet=Image.new('RGB',(1920,780),'#ddd');d=ImageDraw.Draw(sheet)
        for i,t in enumerate(times):
            im=frame(t,a.width);im.save(a.out/f'key-{i+1:02}-{t}.png');im.thumbnail((640,360));x=i%3*640;y=i//3*390;sheet.paste(im,(x,y));d.text((x+10,y+368),f'{t:.2f}s',fill='black')
        sheet.save(a.out/'six-keyframes.jpg',quality=95);return
    dest=a.out/'xcs-branch.mp4'
    p=subprocess.Popen(['ffmpeg','-v','error','-y','-f','rawvideo','-pix_fmt','rgb24','-s',f'{a.width}x{round(a.width*9/16)}','-r','30','-i','-','-c:v','libx264','-preset','veryfast','-crf','18','-pix_fmt','yuv420p','-movflags','+faststart',str(dest)],stdin=subprocess.PIPE)
    for i in range(round(DURATION*30)):p.stdin.write(frame(i/30,a.width).tobytes())
    p.stdin.close()
    if p.wait():raise RuntimeError('ffmpeg failed')
    print(dest)
if __name__=='__main__':main()
