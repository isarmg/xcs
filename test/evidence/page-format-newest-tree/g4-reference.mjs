// Independent article equations: composite Simpson integration, no product imports.
const p=.4, A=Math.PI*(1-p)/2, alpha=Math.PI*p/2;
const F=u=>u**6-3*u**5+2.5*u**4;
function simpson(f,a,b,n=2048){if(a===b)return 0;const h=(b-a)/n;let s=f(a)+f(b);for(let i=1;i<n;i++)s+=(i%2?4:2)*f(a+i*h);return s*h/3;}
const Ix=simpson(u=>Math.cos(A*F(u)),0,1),Iy=simpson(u=>Math.sin(A*F(u)),0,1);
export const K=A*(Ix+Iy)+Math.SQRT2*Math.sin(Math.PI*p/4);
const L=A/K,S=(2*A+alpha)/K,first=u=>[L*simpson(v=>Math.cos(A*F(v)),0,u),L*simpson(v=>Math.sin(A*F(v)),0,u)],end=first(1);
export function g4(t){if(t<=.375)return first(t/.375);if(t>=.625){const v=first((1-t)/.375);return[1-v[1],1-v[0]];}const theta=A/2+K*(S*t-L);return[end[0]+(Math.sin(theta)-Math.sin(A/2))/K,end[1]+(Math.cos(A/2)-Math.cos(theta))/K];}
export const reference=Array.from({length:2001},(_,i)=>g4(i/2000));
const distanceSegment=(p,a,b)=>{const dx=b[0]-a[0],dy=b[1]-a[1],t=Math.max(0,Math.min(1,((p[0]-a[0])*dx+(p[1]-a[1])*dy)/(dx*dx+dy*dy||1)));return Math.hypot(p[0]-a[0]-t*dx,p[1]-a[1]-t*dy);};
const nearestPolyline=(p,points)=>{let best=Infinity;for(let i=1;i<points.length;i++)best=Math.min(best,distanceSegment(p,points[i-1],points[i]));return best;};
export function verifyPath(path,expectedExtent){
 const outer=path.slice(0,path.indexOf('Z')+1);if(!/^[MLZ0-9.eE+ \-]+$/.test(outer))throw new Error('Unexpected path commands for the rendered corner');
 const nums=outer.match(/[-+]?(?:\d*\.)?\d+(?:[eE][-+]?\d+)?/g).map(Number),points=[];for(let i=0;i<nums.length;i+=2)points.push([nums[i],nums[i+1]]);
 const width=Math.max(...points.map(q=>q[0])),height=Math.max(...points.map(q=>q[1])),extent=points[0][0];
 if(Math.abs(extent-expectedExtent)>1e-4)throw new Error(`Rendered extent ${extent} != resolved CSS extent ${expectedExtent}`);
 const group=(points.length-1)/4;if(!Number.isInteger(group)||group<4)throw new Error('A complete rendered four-corner contour is required');
 const errors=[],counts=[];
 for(let c=0;c<4;c++){
  const actual=points.slice(1+c*group,1+(c+1)*group).map(([x,y])=>c===0?[x-(width-extent),y]:c===1?[y-(height-extent),width-x]:c===2?[extent-x,height-y]:[extent-y,x]);
  const scaled=reference.map(([x,y])=>[x*extent,y*extent]);
  const vertexError=Math.max(...actual.map(q=>nearestPolyline(q,scaled))),chordError=Math.max(...scaled.map(q=>nearestPolyline(q,actual)));
  if(Math.max(vertexError,chordError)>.02)throw new Error(`Rendered corner ${c} differs from independently integrated G4 by ${Math.max(vertexError,chordError)}px`);
  errors.push({corner:c,vertexError,chordError});counts.push(actual.length);
 }
 return{width,height,extent,pointCount:points.length,pointsPerCorner:counts,maximumPhysicalError:Math.max(...errors.flatMap(e=>[e.vertexError,e.chordError])),errors,firstTransitionJoin:g4(.375),secondTransitionJoin:g4(.625)};
}
