#!/usr/bin/env python3
"""Object-led XCS explanation. Original paper/storage animation plus real evidence."""
import argparse,json,math,subprocess
from pathlib import Path
from functools import lru_cache
from PIL import Image,ImageDraw
from render import Canvas,BLUE,INK,MUTED,BORDER,GREEN,ease,lerp,text_tile

HERE=Path(__file__).resolve().parent
DURATION=54

@lru_cache(maxsize=4)
def capture_image(path):return Image.open(path).convert('RGB')
def fit_capture(path,crop,w,h):
    source=capture_image(str(path));ratio=source.width/1920
    return source.crop(tuple(round(v*ratio) for v in crop)).resize((w,h),Image.Resampling.LANCZOS)
def disk(c,x,y,name,content=None,state='idle'):
    c.box((x,y,x+280,y+260),'#d9dde4',r=14)
    c.box((x+8,y+8,x+272,y+252),'#edf0f4','#bac2ce',r=11,width=2)
    c.box((x+25,y+22,x+255,y+180),'#fafbfc',BORDER,r=8)
    c.text(x+140,y+41,name,28,INK,True,anchor='mt')
    c.line([(x+30,y+210),(x+215,y+210)],'#a1a8b3',5)
    c.circle(x+242,y+210,7,GREEN if state=='ready' else '#a1a1aa')


def tween(t,a,b):return ease((t-a)/(b-a))
def mix(a,b,q):return tuple(lerp(x,y,q) for x,y in zip(a,b))
@lru_cache(maxsize=8)
def paper(number):
    c=Canvas(1920,1080)
    c.box((0,0,480,570),'#fffefa','#bdc7d5',r=4,width=2)
    c.d.polygon([(675,0),(720,45),(675,45)],fill='#dbe3ec')
    c.text(35,32,'活动方案.txt',33,INK,True)
    c.line([(35,93),(445,93)],'#d5dce5',2)
    c.text(35,135,'活动：秋季展览',29,MUTED)
    c.text(35,208,'预算',30,INK)
    c.text(30,263,str(number),139,BLUE,True)
    nw=text_tile(str(number),round(139*c.s),BLUE,True).width/c.s
    c.text(30+nw+28,352,'万元',35,INK)
    c.text(35,465,'地点：一号展厅',29,MUTED)
    return c.im.crop((0,0,720,855))

def doc(c,n,rect,opacity=1,active=False):
    if opacity<=0:return
    x,y,w=rect;h=w*570/480;s=c.s
    # Several close paper edges convey physical thickness, not extra versions.
    for depth in [12,9,6,3]:
        c.box((x+depth*.6,y+depth,x+w+depth*.6,y+h+depth),'#dce2e9' if depth%2 else '#c7d0dc','#aab8c9',r=2,width=1)
    im=paper(n).resize((max(1,round(w*s)),max(1,round(h*s))),Image.Resampling.LANCZOS)
    if opacity<1:im=Image.blend(c.im.crop((round(x*s),round(y*s),round(x*s)+im.width,round(y*s)+im.height)),im,opacity)
    c.im.paste(im,(round(x*s),round(y*s)))
    if active:c.line([(x+12,y+h+24),(x+w-12,y+h+24)],BLUE,4)

def tag(c,x,y,text):c.text(x,y,text,29,BLUE,True,anchor='mt')
def path(c,points,q,color=BLUE,width=4):
    # Reveal an actual spatial relation, not a decorative traveling light.
    lengths=[math.dist(a,b) for a,b in zip(points,points[1:])];remaining=sum(lengths)*q
    for a,b,length in zip(points,points[1:],lengths):
        if remaining<=0:break
        r=min(1,remaining/length);c.line([a,mix(a,b,r)],color,width);remaining-=length

A=(400,70,480);H8=(95,150,300);H10=(490,150,300);H12=(885,150,300)
OLD=(100,205,300);MAIN=(865,65,270);BRANCH=(865,390,240)
PAIR12=(160,160,370);PAIR6=(750,160,370)

def actors(c,t):
    bq=tween(t,35,37);bg=tuple(round(lerp(a,b,bq)) for a,b in zip((237,242,248),(244,239,230)))
    c.box((0,0,1280,720),bg,r=0)
    if t<9:
        # Each edit leaves the prior complete file on the timeline.
        q=tween(t,2.3,4.2);r=tween(t,5.5,7.8)
        p8=mix(A,H8,q);doc(c,8,p8,active=t<2.3)
        if t>=2.3:
            p10=mix(A,H10,r);doc(c,10,p10,tween(t,2.3,2.8),active=2.8<t<5.5)
        if t>=5.5:doc(c,12,mix(A,H12,r),tween(t,5.5,6.0),active=True)
        for x,at in [(245,3.8),(640,7),(1035,7.8)]:
            if t>=at:c.circle(x,570,7,BLUE)
        if t>4:path(c,[(245,570),(640,570),(1035,570)],tween(t,4,8.5),'#9aabc3',3)
    elif t<18:
        q=tween(t,9,11.2)
        doc(c,10,mix(H10,(600,30,120),q))
        doc(c,12,mix(H12,MAIN,q));doc(c,8,mix(H8,OLD,q),active=True)
        # Selection briefly targets the current budget, then retreats to the old
        # whole page: the latest version is deliberately left unchanged.
        if 9<=t<12:
            px,py,pw=mix(H12,MAIN,q)
            latest=(px+pw*.055,py+pw*.54,px+pw*.57,py+pw*.86)
            old=(OLD[0]-5,OLD[1]-5,OLD[0]+OLD[2]+5,OLD[1]+OLD[2]*570/480+5)
            c.box(mix(latest,old,tween(t,9.6,11.6)),None,BLUE,r=2,width=2)
        if 15.8<t<18:
            focus=math.sin(math.pi*tween(t,15.8,18))**2
            c.box((OLD[0]-8*focus,OLD[1]-8*focus,OLD[0]+OLD[2]+8*focus,OLD[1]+OLD[2]*570/480+8*focus),None,BLUE,r=3,width=2)
        if q>.9:tag(c,1000,20,'主方案')
        p=tween(t,12,15)
        path(c,[(400,385),(655,385),(655,535),(970,535)],p)
        if p>.99:
            c.d.ellipse((947*c.s,512*c.s,993*c.s,558*c.s),outline=BLUE,width=round(3*c.s))
            tag(c,970,lerp(600,665,tween(t,17.3,18)),'精简方案')
        # The branch endpoint is empty. No second body is created here.
    elif t<27:
        q=tween(t,23.5,27)
        path(c,[(400,385),(655,385),(655,535),(970,535)],1-q)
        doc(c,10,mix((600,30,120),(-250,30,120),q))
        doc(c,8,mix(OLD,(-450,205,300),q))
        doc(c,12,mix(MAIN,PAIR12,q))
        tag(c,lerp(1000,345,q),lerp(20,105,q),'主方案')
        # First branch write: a full new 6 page unfolds at the empty endpoint.
        birth=tween(t,21,23.3)
        if birth>0:
            rect=mix((965,530,10),BRANCH,birth);rect=mix(rect,PAIR6,q);doc(c,6,rect)
        else:
            c.d.ellipse((947*c.s,512*c.s,993*c.s,558*c.s),outline=BLUE,width=round(3*c.s))
            # A temporary edit line, not a second saved body.
            c.text(745,590,'预算',28,INK)
            c.text(845,568,'8' if t<19.5 else '6',64,BLUE,True)
            c.text(910,595,'万元',25,INK)
            if t<19.5:c.line([(840,611),(890,611)],'#b45309',3)
            else:c.line([(894,577),(894,630)],BLUE,2)
        if birth>0:
            # Wait until the whole page has separated from the main document.
            if t>=26:
                alpha=tween(t,26,26.5)
                color=tuple(round(lerp(a,b,alpha)) for a,b in zip((237,242,248),(37,99,235)))
                c.text(rect[0]+rect[2]/2,rect[1]-55,'精简方案',29,color,True,anchor='mt')
        else:tag(c,985,665,'精简方案')
    elif t<36:
        pulse1=math.sin(math.pi*tween(t,27,31))**2
        pulse2=math.sin(math.pi*tween(t,31,35.5))**2
        p12=mix(PAIR12,(100,90,450),pulse1);p6=mix(PAIR6,(690,90,450),pulse2)
        doc(c,12,p12,active=pulse1>.2);doc(c,6,p6,active=pulse2>.2)
        tag(c,p12[0]+p12[2]/2,p12[1]-55,'主方案');tag(c,p6[0]+p6[2]/2,p6[1]-55,'精简方案')
    elif t<48:
        q=tween(t,36,38);put=tween(t,38,40.5);copy=tween(t,42,45.2);ret=tween(t,45.2,48)
        mainrect=mix(mix(PAIR12,(38,25,130),q),PAIR12,ret);doc(c,12,mainrect)
        tag(c,mainrect[0]+mainrect[2]/2,mainrect[1]-55,'主方案')
        dy=ret*520+(1-q)*520
        disk(c,230,350+dy,'主盘',None,'ready' if put>.99 else 'idle')
        disk(c,790,350+dy,'另一块盘',None,'ready' if copy>.99 else 'idle')
        if put<1:
            page=mix(mix(PAIR6,(430,35,330),q),(305,402,126),put);doc(c,6,page)
            if q<1:tag(c,page[0]+page[2]/2,page[1]-55,'精简方案')
        else:doc(c,6,(305,402+dy,126))
        if t>=42 and copy<1:
            rect=mix((305,402,126),(865,402,126),copy)
            doc(c,6,(rect[0],rect[1]-160*math.sin(copy*math.pi),rect[2]))
        if copy>=1:doc(c,6,mix((865,402,126),PAIR6,ret))
        if ret>.5:
            tag(c,345,105,'主方案');tag(c,935,105,'精简方案')
    else:
        # Same two complete versions remain available at the end.
        doc(c,12,PAIR12);doc(c,6,PAIR6)
        tag(c,345,105,'主方案');tag(c,935,105,'精简方案')
    return c.im

def frame(t,w,capture):
    c=Canvas(w,round(w*9/16));actors(c,t)
    if 48<=t<51.6:
        # Storage has already left the screen. The real screenshots substantiate
        # only the two branch bodies, never an untested physical-copy result.
        q=tween(t,48,48.7)*(1-tween(t,50.9,51.6))
        c.box((0,0,1280,720),'#f4efe6',r=0)
        for number,name,pair,title in [(12,'14-main-still-12-preview.png',PAIR12,'主方案'),(6,'12-branch-6-preview.png',PAIR6,'精简方案')]:
            doc(c,number,pair,1-q)
            x,y,pw=pair;ph=pw*570/480
            target=(x+pw/2-285,y+ph/2-109.15,570,218.3)
            x,y,pw,ph=mix((x,y,pw,ph),target,q);s=w/1280
            ih=min(ph,pw*193/504);iw=ih*504/193;ix=x+(pw-iw)/2;iy=y+(ph-ih)/2
            im=fit_capture(capture/name,[708,0,1212,193],max(1,round(iw*s)),max(1,round(ih*s)))
            if q<1:im=Image.blend(c.im.crop((round(ix*s),round(iy*s),round(ix*s)+im.width,round(iy*s)+im.height)),im,q)
            c.im.paste(im,(round(ix*s),round(iy*s)))
            tag(c,pair[0]+pair[2]/2,105,title)
    return c.im

def main():
    p=argparse.ArgumentParser();p.add_argument('--capture',type=Path,default=HERE/'assets/capture');p.add_argument('--out',type=Path,required=True);p.add_argument('--width',type=int,default=1920);p.add_argument('--render',action='store_true');p.add_argument('--duration',type=float,default=DURATION);a=p.parse_args();a.out.mkdir(parents=True,exist_ok=True)
    if not a.render:
        thumbs=[]
        for t in [1,4.8,8.5,11.5,16,20.8,25.5,29,33,39.5,43.5,46.5,49.5,53]:
            im=frame(t,a.width,a.capture);im.save(a.out/f'object-{t}.jpg',quality=95);im.thumbnail((480,270));thumbs.append((t,im))
        sheet=Image.new('RGB',(1920,1200),'#ddd');d=ImageDraw.Draw(sheet)
        for i,(t,im) in enumerate(thumbs):x=i%4*480;y=i//4*300;sheet.paste(im,(x,y));d.text((x+8,y+276),str(t),fill='black')
        sheet.save(a.out/'object-contact.jpg',quality=95);return
    if not (a.capture/'verification.txt').exists():raise RuntimeError('Missing verified real-capture evidence')
    dest=a.out/('object-motion-preview.mp4' if a.duration<DURATION else 'xcs-v3-content-silent.mp4')
    cmd=['ffmpeg','-v','error','-y','-f','rawvideo','-pix_fmt','rgb24','-s',f'{a.width}x{round(a.width*9/16)}','-r','30','-i','-','-c:v','libx264','-preset','veryfast','-crf','18','-pix_fmt','yuv420p','-movflags','+faststart',str(dest)]
    proc=subprocess.Popen(cmd,stdin=subprocess.PIPE)
    for i in range(round(a.duration*30)):
        proc.stdin.write(frame(i/30,a.width,a.capture).tobytes())
        if i%300==0:print(i/30,flush=True)
    proc.stdin.close()
    if proc.wait():raise RuntimeError('ffmpeg failed')
    print(dest)
if __name__=='__main__':main()
