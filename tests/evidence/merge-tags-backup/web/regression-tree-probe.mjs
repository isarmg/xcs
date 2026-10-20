// Historical evidence probe, not current acceptance. See tests/evidence/README.md.
import {configureProbe, loadPlaywright, fixtureOrigin, requiredInput} from '../../_support/runtime.mjs';
const historicalOrigin = fixtureOrigin();
const probe = await configureProbe(import.meta.url);
// Actual production WASM, isolated static hosting and a fully intercepted API.
import assert from 'node:assert/strict';
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';

const {chromium}=await loadPlaywright();
const root=probe.root;
const base=historicalOrigin;
const output=probe.output;
const browser=await chromium.launch({headless:true,args:['--no-sandbox']});
const scenes=[];
const records=[
  {version_id:'root',created_at:'1',source_version_ids:[],note:'Root'},
  {version_id:'left',created_at:'2',source_version_ids:['root'],note:'Left'},
  {version_id:'right',created_at:'3',source_version_ids:['root'],note:'A deliberately taller sibling with a long historical note. '.repeat(4)},
  {version_id:'left-tip',created_at:'4',source_version_ids:['left'],note:'Left tip'},
  {version_id:'short-tip',created_at:'5',source_version_ids:['root'],note:'Short branch'},
  {version_id:'merge',created_at:'6',source_version_ids:['left-tip','right'],note:'Merge',current:true},
  {version_id:'unrelated',created_at:'0',source_version_ids:[],note:'Independent'}
].map(version=>({...version,file_id:'fixture-file',content_state:'LIVE'}));
const expectedEdges=records.flatMap(version=>version.source_version_ids.map(parent=>`${parent}>${version.version_id}`)).sort();
await mkdir(output,{recursive:true});

async function inspect(page,label) {
  await page.waitForFunction(expected=>document.querySelectorAll('.zxs-version-tree-edge[d^="M"]').length===expected,expectedEdges.length);
  await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
  const geometry=await page.evaluate(()=>{
    const canvas=document.querySelector('.zxs-version-tree-canvas'),frame=canvas.getBoundingClientRect();
    const nodes=[...canvas.querySelectorAll('.zxs-version-node')].map(node=>{
      const rect=node.getBoundingClientRect();
      return {id:node.dataset.versionId,left:rect.left-frame.left,right:rect.right-frame.left,top:rect.top-frame.top,bottom:rect.bottom-frame.top};
    });
    const edges=[...canvas.querySelectorAll('.zxs-version-tree-edge')].map(edge=>{
      const style=getComputedStyle(edge),length=edge.getTotalLength(),points=[];
      for(let distance=0;distance<length;distance+=.75){const point=edge.getPointAtLength(distance);points.push([point.x,point.y]);}
      const end=edge.getPointAtLength(length);points.push([end.x,end.y]);
      return {parent:edge.dataset.parentVersion,child:edge.dataset.childVersion,d:edge.getAttribute('d'),points,strokeWidth:style.strokeWidth,dash:style.strokeDasharray,cap:style.strokeLinecap,join:style.strokeLinejoin,vector:style.vectorEffect,marker:edge.getAttribute('marker-end')};
    });
    return {nodes,edges};
  });
  assert.equal(geometry.nodes.length,7,label);
  assert.deepEqual(geometry.edges.map(edge=>`${edge.parent}>${edge.child}`).sort(),expectedEdges,label);
  const byId=new Map(geometry.nodes.map(node=>[node.id,node]));
  let rounded=0,crossLayer=0;
  for(const edge of geometry.edges){
    const parent=byId.get(edge.parent),child=byId.get(edge.child);
    assert.ok(parent.top>child.bottom,label+': parent remains below child');
    assert.ok(Math.abs(edge.points[0][0]-(parent.left+parent.right)/2)<.05);
    assert.ok(Math.abs(edge.points[0][1]-parent.top)<.05);
    assert.ok(Math.abs(edge.points.at(-1)[0]-(child.left+child.right)/2)<.05);
    assert.ok(Math.abs(edge.points.at(-1)[1]-(child.bottom+3))<.05);
    assert.equal(edge.strokeWidth,'1.5px');
    assert.equal(edge.dash,'none');
    assert.equal(edge.cap,'round');
    assert.equal(edge.join,'round');
    assert.equal(edge.vector,'non-scaling-stroke');
    assert.equal(edge.marker,'url(#zxs-version-arrow)');
    const tokens=edge.d.match(/[A-Za-z]|[-+]?(?:\d*\.\d+|\d+)(?:e[-+]?\d+)?/g);
    let x,y;
    for(let index=0;index<tokens.length;){
      const command=tokens[index++];
      assert.ok(['M','H','V','A'].includes(command),label+': all straight runs use H/V, corners use circular A');
      if(command==='M'){x=Number(tokens[index++]);y=Number(tokens[index++]);}
      if(command==='H')x=Number(tokens[index++]);
      if(command==='V')y=Number(tokens[index++]);
      if(command==='A'){
        const [rx,ry,rotation,large,sweep,nx,ny]=tokens.slice(index,index+7).map(Number);index+=7;
        assert.ok(rx>0&&rx<=4.001&&Math.abs(rx-ry)<.001);
        assert.equal(rotation,0);assert.equal(large,0);assert.ok(sweep===0||sweep===1);
        assert.ok(Math.abs(Math.abs(nx-x)-rx)<.002&&Math.abs(Math.abs(ny-y)-ry)<.002,label+': quarter-circle corner');
        x=nx;y=ny;rounded++;
      }
    }
    for(let index=0;index<edge.points.length;index++){
      const [x,y]=edge.points[index];
      if(index)assert.ok(y<=edge.points[index-1][1]+.002,label+': route always travels upward');
      for(const node of geometry.nodes)assert.ok(!(x>node.left+.02&&x<node.right-.02&&y>node.top+.02&&y<node.bottom-.02),label+': connector avoids node interior '+node.id);
    }
    if(edge.points.some(([x])=>x<Math.min(...geometry.nodes.map(node=>node.left))||x>Math.max(...geometry.nodes.map(node=>node.right))))crossLayer++;
  }
  assert.ok(rounded>=4,label+': fork/merge paths exercise rounded turns');
  assert.ok(crossLayer>=1,label+': deep ancestry exercises outside lanes');
  const summary={label,nodes:geometry.nodes,edges:geometry.edges.map(({points,...edge})=>({...edge,samples:points.length})),roundedTurns:rounded,crossLayerEdges:crossLayer};
  await writeFile(`${output}/${label}.json`,JSON.stringify(summary,null,2)+'\n');
  return summary;
}

try{
  for(const [language,width,theme]of[['zh',1440,'light'],['en',390,'dark']]){
    const key=`${language}-${theme}-${width}`,state={key,checks:[],unexpected:[],pageErrors:[],wasm:[]};
    const context=await browser.newContext({viewport:{width,height:1100},colorScheme:theme}),page=await context.newPage(),pending=[];
    const labels=language==='zh'?{edit:'编辑备注',note:'版本备注'}:{edit:'Edit note',note:'Version note'};
    await page.addInitScript(theme=>localStorage.setItem('zxs-theme',theme),theme);
    page.on('pageerror',error=>state.pageErrors.push(error.message));
    page.on('response',response=>{
      const path=new URL(response.url()).pathname;
      if(path.endsWith('.wasm'))pending.push((async()=>{const body=await response.body();state.wasm.push({path,status:response.status(),sha256:createHash('sha256').update(body).digest('hex'),matchesCandidate:body.equals(await readFile(`${root}/web/dist${path}`))});})());
    });
    const entry={entry_id:'fixture-entry',file_id:'fixture-file',kind:'file',name:'routes.txt',path:'routes.txt',version_id:'merge',materialized:true,inherited:false,own_version_id:'merge',selected_branch_id:'main',size:'0'};
    await page.route('**/v1/**',async route=>{
      const request=route.request(),path=new URL(request.url()).pathname;
      let body;
      if(request.method()==='GET'&&path==='/v1/auth/me')body={principal:'fixture-user',admin:false,csrf_token:'fixture-only'};
      else if(request.method()==='GET'&&path==='/v1/file-manager/disks')body={disks:[{slot_id:'P01',online:true,writable:true,available_bytes:'4096'}]};
      else if(request.method()==='GET'&&path==='/v1/file-manager/disks/P01/entries')body={entries:[entry],writable:true,directory_branches:[],directory_branches_supported:true,directory_file_count:'1'};
      else if(request.method()==='GET'&&path==='/v1/file-manager/files/fixture-file/history')body={...entry,note:'',versions:records,branches:[{branch_id:'main',name:'main',current:true,version_id:'merge',materialized:true}]};
      else{state.unexpected.push({method:request.method(),path});body={code:'UNEXPECTED_FIXTURE_API',message:'Every API request is intercepted'};}
      return route.fulfill({status:state.unexpected.length?500:200,contentType:'application/json',body:JSON.stringify(body)});
    });
    try{
      await page.goto(`${base}/?lang=${language}`);
      await page.locator('html[data-zxs-ready="true"]').waitFor();
      await page.locator('.zxs-disk-entry[data-slot="P01"]').click();
      await page.getByRole('button',{name:'routes.txt',exact:true}).click();
      state.checks.push(await inspect(page,`default-${key}`));
      await page.screenshot({path:`${output}/default-${key}.png`,fullPage:true});
      await page.locator('.zxs-version-node[data-version-id="right"]').getByRole('button',{name:labels.edit,exact:true}).click();
      await page.locator('.zxs-version-node[data-version-id="right"] input[type="text"]').waitFor();
      state.checks.push(await inspect(page,`editing-${key}`));
      await page.locator('.zxs-version-node[data-version-id="right"] input[type="text"]').press('Escape');
      await page.setViewportSize({width:width-40,height:1100});
      state.checks.push(await inspect(page,`resized-${key}`));
      await page.screenshot({path:`${output}/resized-${key}.png`,fullPage:true});
      await Promise.all(pending);
      assert.ok(state.wasm.length&&state.wasm.every(wasm=>wasm.status===200&&wasm.matchesCandidate));
      assert.deepEqual(state.unexpected,[]);assert.deepEqual(state.pageErrors,[]);
      state.passed=true;
    }catch(error){state.passed=false;state.error=String(error.stack||error);await page.screenshot({path:`${output}/failure-${key}.png`,fullPage:true}).catch(()=>{});}
    await context.close();scenes.push(state);
    await writeFile(`${output}/result-${key}.json`,JSON.stringify(state,null,2)+'\n');
    console.log(`${key}: ${state.passed?'PASS':'FAIL'} (${state.checks.length} geometry states)`);
  }
}finally{await browser.close();}
await writeFile(`${output}/browser-result.json`,JSON.stringify({passed:scenes.every(scene=>scene.passed),scenes},null,2)+'\n');
assert.ok(scenes.every(scene=>scene.passed),scenes.filter(scene=>!scene.passed).map(scene=>scene.error).join('\n'));
