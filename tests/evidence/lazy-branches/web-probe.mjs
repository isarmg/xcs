// Real candidate WASM with an entirely intercepted, deterministic API fixture.
// No authentication or business request reaches the running preview service.
import assert from 'node:assert/strict';
import {readFile, writeFile, mkdir} from 'node:fs/promises';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';

const {chromium} = await import(process.env.XCS_PLAYWRIGHT_MODULE || 'playwright');
const root = resolve(process.env.XCS_PRODUCT_ROOT || fileURLToPath(new URL('../../../', import.meta.url)));
const base = process.env.XCS_PREVIEW_URL || 'http://127.0.0.1:7400';
const output = resolve(process.env.XCS_BRANCH_EVIDENCE || `${root}/tests/evidence/lazy-branches`);
const emptyHash = 'af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262';
const texts = {
  zh: {branch:'创建分支', branchName:'分支名称', current:'当前分支', pending:'待写入', preview:'预览', download:'下载', currentCreate:'从当前内容创建分支', write:'写入当前分支'},
  en: {branch:'Create branch', branchName:'Branch name', current:'Current branch', pending:'Pending content', preview:'Preview', download:'Download', currentCreate:'Branch from current content', write:'Write to current branch'}
};
const browser = await chromium.launch({headless:true,args:['--no-sandbox']});
const results = [];
await mkdir(output,{recursive:true});

async function scenario(language,width,theme) {
  const key = `${language}-${theme}-${width}`;
  const labels = texts[language];
  const context = await browser.newContext({viewport:{width,height:1050},colorScheme:theme});
  const page = await context.newPage();
  page.setDefaultTimeout(20000);
  const state = {key,checks:[],mockRequests:[],unexpected:[],pageErrors:[],assets:[],contentReads:0};
  const reads = [];
  let selected = 'main', draft = null, written = false;
  await page.addInitScript(theme => localStorage.setItem('xcs-theme',theme),theme);
  page.on('pageerror',error=>state.pageErrors.push(error.message));
  page.on('response',response=>{
    const path = new URL(response.url()).pathname;
    if (path.endsWith('.wasm')) reads.push((async()=>{
      const body=await response.body();
      const local=await readFile(`${root}/web/dist${path}`);
      state.assets.push({path,status:response.status(),mime:response.headers()['content-type'],sha256:createHash('sha256').update(body).digest('hex'),matchesCandidate:body.equals(local)});
    })());
  });
  const version = () => selected === 'main' ? 'source' : written ? 'first-write' : null;
  const entry = () => ({entry_id:'entry-file',file_id:'file-a',kind:'file',name:'report.txt',path:'report.txt',selected_branch_id:selected,version_id:version(),materialized:version()!==null,content_state:version()===null?'PENDING':'LIVE',source_version_id:selected==='main'?null:'source',size:selected==='main'?'6':'0'});
  const branches = () => [{branch_id:'main',name:'main',current:selected==='main',version_id:'source',materialized:true,content_state:'LIVE'},...(draft?[{branch_id:'draft',name:draft,current:selected==='draft',version_id:written?'first-write':null,materialized:written,content_state:written?'LIVE':'PENDING',source_version_id:'source'}]:[])];
  const versions = () => [{file_id:'file-a',version_id:'source',created_at:'1',size:'6',source_version_ids:[],note:'source only',current:selected==='main',content_state:'LIVE'},...(written?[{file_id:'file-a',version_id:'first-write',created_at:'2',size:'0',source_version_ids:['source'],note:'',current:selected==='draft',content_state:'LIVE',materialized:true}]:[])];
  const published = (request,result={}) => ({operation_id:request.operation_id,state:'PUBLISHED',result});
  await page.route('**/v1/**',async route=>{
    const request=route.request(),url=new URL(request.url()),path=url.pathname,method=request.method();
    const json=()=>request.postDataJSON() || {};
    const fulfill=(body,status=200)=>route.fulfill({status,contentType:'application/json',body:JSON.stringify(body)});
    state.mockRequests.push({method,path});
    if(method==='GET'&&path==='/v1/auth/me')return fulfill({principal:'fixture-user',admin:false,csrf_token:'fixture-only'});
    if(method==='GET'&&path==='/v1/file-manager/disks')return fulfill({disks:[{slot_id:'P01',online:true,writable:true,available_bytes:'4096'}]});
    if(method==='GET'&&path==='/v1/file-manager/disks/P01/entries')return fulfill({entries:[entry()],writable:true,directory_branches_supported:true,directory_file_count:'1',directory_branches:branches()});
    if(method==='GET'&&path==='/v1/file-manager/files/file-a/history')return fulfill({file_id:'file-a',path:'report.txt',note:'',selected_branch_id:selected,branches:branches(),versions:versions()});
    if(method==='POST'&&path==='/v1/file-manager/files/file-a/branches') {
      const body=json();
      assert.equal(body.source_version_id,'source');
      draft=body.name;
      return fulfill(published(body,{branch_id:'draft',copied_files:'0',placeholder_files:'1',materialized:false,version_id:null}));
    }
    if(method==='POST'&&path==='/v1/file-manager/files/file-a/select') {
      const body=json();
      assert.equal(body.branch_id,'draft');
      selected=body.branch_id;
      return fulfill(published(body,{branch_id:selected}));
    }
    if(method==='POST'&&path==='/v1/file-manager/disks/P01/write-sessions') {
      state.writeTarget=json();
      assert.deepEqual(state.writeTarget,{path:'report.txt',branch_id:'draft'});
      return fulfill({workspace_id:'write-pending',base_head:'base-commit',relative_path:'report.txt'});
    }
    if(method==='PUT'&&/^\/v1\/workspaces\/write-pending\/files\/[^/]+\/content$/.test(path)) {
      state.uploadBytes=request.postDataBuffer()?.length || 0;
      assert.equal(url.searchParams.get('path'),'report.txt');
      assert.equal(state.uploadBytes,0);
      return fulfill({saved:true});
    }
    if(method==='POST'&&path==='/v1/workspaces/write-pending/seal')return fulfill({workspace_id:'write-pending',manifest_digest:'fixture-seal'});
    if(method==='POST'&&path==='/v1/workspaces/write-pending/commit') {
      const body=json();
      assert.equal(body.expected_head,'base-commit');
      assert.match(body.request_digest,/^[a-f0-9]{64}$/);
      written=true;
      return fulfill(published(body,{commit_id:'first-commit',branch_id:'draft'}));
    }
    if(method==='GET'&&path==='/v1/versions/first-write/content') {
      state.contentReads++;
      return route.fulfill({status:200,headers:{'Content-Type':'text/plain','Content-Length':'0',ETag:`"${emptyHash}"`},body:Buffer.alloc(0)});
    }
    state.unexpected.push({method,path});
    return fulfill({code:'FIXTURE_UNEXPECTED_REQUEST',message:'All API traffic is intercepted',retryable:false},500);
  });
  try {
    await page.goto(`${base}/?lang=${language}`);
    await page.locator('html[data-xcs-ready="true"]').waitFor();
    await page.locator('.xcs-disk-entry[data-slot="P01"]').click();
    await page.getByRole('button',{name:'report.txt',exact:true}).click();
    await page.locator('.xcs-version-node[data-version-id="source"]').waitFor();
    assert.equal(await page.locator('.xcs-version-node').count(),1);
    await page.locator('.xcs-version-node').getByRole('button',{name:labels.branch,exact:true}).click();
    const editor=page.getByRole('textbox',{name:labels.branchName,exact:true});
    await editor.fill('draft-empty');
    await editor.press('Enter');
    const select=page.getByRole('combobox',{name:labels.current,exact:true});
    await page.waitForFunction(()=>document.querySelector('.xcs-file-branches [role="combobox"]')?.disabled===false);
    await select.click();
    await page.getByRole('option',{name:`draft-empty · ${labels.pending}`,exact:true}).click();
    await page.waitForFunction(label=>document.querySelector('.xcs-file-branches [role="combobox"]')?.textContent?.includes(label),labels.pending);
    assert.ok(await page.getByRole('button',{name:labels.preview,exact:true}).isDisabled());
    assert.ok(await page.getByRole('button',{name:labels.download,exact:true}).isDisabled());
    assert.ok(await page.getByRole('button',{name:labels.currentCreate,exact:true}).isDisabled());
    assert.equal(await page.locator('.xcs-version-node').count(),1);
    assert.equal(await page.locator('.xcs-version-current').count(),0);
    assert.ok(await page.locator('.xcs-version-node[data-version-id="source"]').getByRole('button',{name:labels.branch,exact:true}).isEnabled());
    assert.equal(state.contentReads,0);
    state.checks.push('Created branch stores no content, pending selection cannot read its source, and no placeholder tree node is invented');
    await page.screenshot({path:`${output}/pending-${key}.png`,fullPage:true});

    await page.getByRole('button',{name:labels.write,exact:true}).waitFor({state:'visible'});
    assert.ok(await page.getByRole('button',{name:labels.write,exact:true}).isEnabled());
    await page.locator('.xcs-entry-content input[type="file"]').setInputFiles({name:'different-local-name.bin',mimeType:'application/octet-stream',buffer:Buffer.alloc(0)});
    await page.locator('.xcs-version-node[data-version-id="first-write"]').waitFor();
    assert.equal(await page.locator('.xcs-version-node').count(),2);
    assert.equal(await page.getByRole('button',{name:labels.preview,exact:true}).isEnabled(),true);
    assert.equal(await page.getByRole('button',{name:labels.download,exact:true}).isEnabled(),true);
    assert.equal(await page.getByRole('button',{name:labels.currentCreate,exact:true}).isEnabled(),true);
    assert.equal((await select.innerText()).includes(labels.pending),false);
    assert.equal(await page.locator('.xcs-file-heading h2').innerText(),'report.txt');
    const order=await page.locator('.xcs-version-node').evaluateAll(nodes=>nodes.map(node=>node.dataset.versionId));
    assert.deepEqual(order,['first-write','source']);
    const edge=page.locator('.xcs-version-tree-edge[data-parent-version="source"][data-child-version="first-write"]');
    // A straight vertical SVG stroke has a zero-width geometry box; Playwright's
    // generic visibility predicate is unsuitable for deciding if it is drawn.
    await edge.waitFor({state:'attached'});
    assert.match(await edge.getAttribute('d'),/^M.+V/);
    state.checks.push('First zero-byte write targets original managed path and captured branch despite a different local filename, then creates one real version above its source');
    await page.getByRole('button',{name:labels.preview,exact:true}).click();
    await page.getByRole('dialog',{name:'report.txt',exact:true}).waitFor();
    assert.equal(state.contentReads,1);
    state.checks.push('Materialized zero-byte content becomes available for verified preview');
    await page.screenshot({path:`${output}/written-${key}.png`,fullPage:true});
    await Promise.all(reads);
    assert.ok(state.assets.length>0 && state.assets.every(asset=>asset.matchesCandidate && asset.status===200 && asset.mime==='application/wasm'));
    assert.deepEqual(state.unexpected,[]);
    assert.deepEqual(state.pageErrors,[]);
    state.passed=true;
  } catch(error) {
    state.passed=false;
    state.error=String(error?.stack || error);
    await page.screenshot({path:`${output}/failure-${key}.png`,fullPage:true}).catch(()=>{});
  } finally {
    await context.close();
    await writeFile(`${output}/web-${key}.json`,JSON.stringify(state,null,2)+'\n');
  }
  results.push(state);
  console.log(`${key}: ${state.passed?'PASS':'FAIL'} (${state.checks.length} behavior groups)`);
}
try {
  for (const [language,width,theme] of [['zh',1440,'light'],['en',390,'dark']]) await scenario(language,width,theme);
} finally {
  await browser.close();
}
await writeFile(`${output}/web-result.json`,JSON.stringify({passed:results.every(result=>result.passed),scenes:results},null,2)+'\n');
assert.ok(results.every(result=>result.passed),results.filter(result=>!result.passed).map(result=>result.error).join('\n'));
