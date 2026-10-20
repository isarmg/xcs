// Render documentation images; no business service or user data is accessed.
import assert from 'node:assert/strict';
import {mkdir,readFile,readdir,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
const {chromium}=await import(process.env.ZXS_PLAYWRIGHT_MODULE||'playwright');
const root=resolve(fileURLToPath(new URL('../../../',import.meta.url)));
const directory=resolve(root,'docs/diagrams/branch-system');
const output=resolve(root,'tests/evidence/merge-tags-backup');
await mkdir(output,{recursive:true});
const names=(await readdir(directory)).filter(name=>/^\d{2}.*\.svg$/.test(name)).sort();
const viewerSource=await readFile(resolve(directory,'index.html'),'utf8');
assert.equal(names.length,[...viewerSource.matchAll(/<article class='panel'/g)].length);
const operationCases={
  '17-move-between-branches.svg':{edges:['M0>M1','L0>L1'],unchanged:['N0','P0']},
  '18-tracking-after-move.svg':{edges:['L1>L2','F1>F2'],unchanged:['M1','N0']},
  '19-batch-move.svg':{edges:['M0>M1','N0>N1','L0>L1','P0>P1'],unchanged:[]},
  '20-batch-copy.svg':{edges:['L0>L1','P0>P1'],unchanged:['M0','Q0']},
  '21-batch-delete.svg':{edges:['M0>M1','N0>N1'],unchanged:['R0']},
  '23-rename-path-nodes.svg':{edges:['D0>D1','F1>P1','G1>Q1'],unchanged:[]},
  '24-file-merge-reuse.svg':{edges:['F1>F2','F1>F3','F2>F3','F3>owner'],unchanged:[]},
  '25-file-merge-chosen.svg':{edges:['F0>F1','F0>F2','F1>M','F2>M','F0>M','M>owner'],unchanged:[]},
  '26-version-tags.svg':{edges:['V1>T','T>W','V1>H','W>head'],unchanged:[]},
  '27-backup-prefix.svg':{edges:[],unchanged:['P','B','C']},
  '22-batch-rename.svg':{edges:['M0>M1','N0>N1'],unchanged:['R0']},
};
for(const name of Object.keys(operationCases))assert.ok(names.includes(name),name);
const browser=await chromium.launch({headless:true,args:['--no-sandbox']});
const result={scope:'Documentation graphics and offline viewer only; no product implementation or business acceptance.',diagrams:[],viewer:[]};
try {
  const context=await browser.newContext({viewport:{width:1500,height:1400},deviceScaleFactor:1});
  const page=await context.newPage();
  const errors=[],externalRequests=[];page.on('pageerror',error=>errors.push(error.message));
  page.on('request',request=>{if(/^https?:/.test(request.url()))externalRequests.push(request.url());});
  for(const name of names){
    await page.goto(pathToFileURL(resolve(directory,name)).href);
    await page.evaluate(()=>document.fonts.ready);
    const shape=await page.evaluate(()=>{
      const svg=document.querySelector('svg');
      const nodes=[...svg.querySelectorAll('.dg-node')].map(node=>({id:node.dataset.node,x:Number(node.dataset.x),y:Number(node.dataset.y),w:Number(node.dataset.width),h:Number(node.dataset.height),head:node.dataset.head==='true'}));
      const edges=[...svg.querySelectorAll('.dg-edge')].map(edge=>{
        const length=edge.getTotalLength(),points=[];
        for(let d=0;d<length;d+=2){const p=edge.getPointAtLength(d);points.push([p.x,p.y]);}
        const p=edge.getPointAtLength(length);points.push([p.x,p.y]);
        return {parent:edge.dataset.parent,child:edge.dataset.child,d:edge.getAttribute('d'),width:getComputedStyle(edge).strokeWidth,dash:getComputedStyle(edge).strokeDasharray,marker:edge.getAttribute('marker-end'),points};
      });
      const overflow=[...svg.querySelectorAll('text')].flatMap(text=>{
        const box=text.getBBox(),node=text.closest('.dg-node');
        if(node){const left=Number(node.dataset.x),right=left+Number(node.dataset.width),top=Number(node.dataset.y),bottom=top+Number(node.dataset.height);if(box.x<left+7||box.x+box.width>right-7||box.y<top||box.y+box.height>bottom)return [{text:text.textContent,box:{x:box.x,y:box.y,width:box.width,height:box.height},node:node.dataset.node}];}
        else if(box.x<0||box.x+box.width>svg.viewBox.baseVal.width)return [{text:text.textContent,outsideSvg:true}];
        return [];
      });
      return {width:svg.viewBox.baseVal.width,height:svg.viewBox.baseVal.height,nodes,edges,overflow,font:getComputedStyle(svg.querySelector('text')).fontFamily};
    });
    assert.deepEqual(shape.overflow,[],`${name}: all text stays inside visible bounds`);
    if(operationCases[name]){
      const expected=operationCases[name];
      assert.deepEqual(shape.edges.map(edge=>`${edge.parent}>${edge.child}`).sort(),expected.edges.toSorted(),`${name}: only the affected branch subtrees advance`);
      for(const id of expected.unchanged){assert.ok(shape.nodes.some(node=>node.id===id));assert.ok(!shape.edges.some(edge=>edge.parent===id||edge.child===id),`${name}: ${id} stays unchanged`);}
    }
    for(const edge of shape.edges){
      const parent=shape.nodes.find(node=>node.id===edge.parent),child=shape.nodes.find(node=>node.id===edge.child);
      assert.ok(parent.y>child.y+child.h,`${name}: parent below child`);
      assert.equal(edge.width,'2.5px');assert.equal(edge.dash,'none');
      assert.ok(edge.marker?.includes('-arrow')||edge.marker?.includes('-ref'));
      assert.deepEqual([...new Set(edge.d.match(/[A-Za-z]/g))].filter(command=>!['M','H','V','A'].includes(command)),[]);
      for(let i=0;i<edge.points.length;i++){
        const [x,y]=edge.points[i];if(i)assert.ok(y<=edge.points[i-1][1]+.01,`${name}: arrow always goes upward`);
        for(const node of shape.nodes)assert.ok(!(x>node.x+.1&&x<node.x+node.w-.1&&y>node.y+.1&&y<node.y+node.h-.1),`${name}: edge avoids ${node.id}`);
      }
    }
    await page.locator('svg').screenshot({path:resolve(directory,name.replace(/\.svg$/,'.png'))});
    result.diagrams.push({file:name,width:shape.width,height:shape.height,nodes:shape.nodes.length,edges:shape.edges.map(({points,...edge})=>edge),textOverflow:shape.overflow,passed:true});
  }
  for(const width of [1600,800,390]){
    await page.setViewportSize({width,height:1150});
    await page.goto(pathToFileURL(resolve(directory,'index.html')).href);
    await page.evaluate(()=>document.fonts.ready);
    assert.equal(await page.locator('script').count(),0);
    assert.equal(await page.locator('.panel:visible svg').getAttribute('data-scene'),'24-file-merge-reuse');
    for(let i=0;i<names.length;i++){
      const label=page.locator(`label[for="case-${i}"]`);await label.scrollIntoViewIfNeeded();await label.click();
      assert.equal(await page.locator('.panel:visible').count(),1);
      assert.ok(await page.locator(`.panel[data-case="${i}"]`).isVisible());
    }
    const moveIndex=names.indexOf('17-move-between-branches.svg');
    await page.locator(`label[for="case-${moveIndex}"]`).scrollIntoViewIfNeeded();await page.locator(`label[for="case-${moveIndex}"]`).click();
    await page.screenshot({path:resolve(output,`viewer-${width}.png`),fullPage:true});
    const overflow=await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth+1);
    assert.equal(overflow,false,`${width}: page width remains bounded; graph scrolls inside its canvas`);
    result.viewer.push({width,tabChecks:names.length,externalRequests:externalRequests.length,scripts:0,passed:true});
  }
  assert.deepEqual(errors,[]);assert.deepEqual(externalRequests,[]);result.passed=true;
  await writeFile(resolve(output,'diagrams-result.json'),JSON.stringify(result,null,2)+'\n');
  console.log(`PASS: ${names.length} rendered diagrams, upward edges, bounded text and ${Object.keys(operationCases).length} operation scopes; ${names.length*3} offline viewer interactions across three widths`);
} finally {await browser.close();}
