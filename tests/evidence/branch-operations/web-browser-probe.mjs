// Historical evidence probe, not current acceptance. See tests/evidence/README.md.
import {configureProbe, loadPlaywright, fixtureOrigin, requiredInput} from '../_support/runtime.mjs';
const probe = await configureProbe(import.meta.url);
// Real production WASM with every request fulfilled locally. This is browser
// interaction evidence; native persistence and API semantics are tested separately.
import assert from 'node:assert/strict';
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import {resolve,extname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
const {chromium}=await loadPlaywright();
const root=probe.root;
const dist=resolve(process.env.ZXS_BROWSER_DIST||`${root}/web/dist`);
const output=probe.output;
const csp="default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; font-src 'self'; img-src 'self' data: blob:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'";
const browser=await chromium.launch({headless:true,args:['--no-sandbox']});
await mkdir(output,{recursive:true});const scenes=[];
const file=(id,path,version)=>({entry_id:id,file_id:id,path,name:path.split('/').at(-1),kind:'file',version_id:version,size:'6',selected_branch_id:`file-${id}`});
const folder={entry_id:'folder',kind:'directory',name:'folder',path:'a/folder'};
const branch=(id,path,entries,current=false)=>({directory_branch_id:id,head_tree_id:`tree-${id}`,name:id,path,entries,current,materialized:true,file_branches:{},inherited_files:[],directory_refs:{}});
try{
for(const [language,width,theme]of[['zh',1440,'light'],['en',390,'dark']]){
const key=`${language}-${width}-${theme}`,scene={key,checks:[],writes:[],unexpected:[],pageErrors:[],wasm:[]};
const labels=language==='zh'?{move:'移动',copy:'复制',rename:'重命名',del:'删除',cancel:'取消',target:'目标目录',name:'目标名称',apply:'执行所选操作',load:'读取目标分支',source:'映射源分支',dest:'映射目标分支',init:'登记源目录分支记录',all:'选择当前显示的全部条目'}:{move:'Move',copy:'Copy',rename:'Rename',del:'Delete',cancel:'Cancel',target:'Destination folder',name:'Destination name',apply:'Apply selected operation',load:'Load destination branches',source:'Mapping source branch',dest:'Mapping destination branch',init:'Register source folder branch records',all:'Select all visible entries'};
const context=await browser.newContext({viewport:{width,height:1100},colorScheme:theme}),page=await context.newPage();page.setDefaultTimeout(15000);
let scopeMissing=false,conflictNext=false,pendingNext=false,pendingId=null,allowReceipt=false,pollCount=0,currentFileBranch='file-f',generatedBranchName='方案甲-7c2a';const sources=[branch('m','a',[file('f','a/f.txt','F1'),file('g','a/g.txt','G1'),folder],true),branch('n','a',[file('f','a/alias.txt','F2'),file('g','a/g.txt','G2'),folder]),branch('r','a',[file('f','a/f.txt','F1')])];
const targets=[branch('l','b',[],true),branch('p','b',[]),branch('q','b',[])];
await page.addInitScript(theme=>localStorage.setItem('zxs-theme',theme),theme);
page.on('pageerror',error=>scene.pageErrors.push(error.message));
await page.route('**/*',async route=>{
const request=route.request(),url=new URL(request.url()),path=url.pathname,method=request.method();
if(!path.startsWith('/v1/')){
 const asset=resolve(dist,path==='/'?'index.html':`.${path}`);assert.ok(asset.startsWith(dist+'/'));
 try{const body=await readFile(asset);if(path.endsWith('.wasm'))scene.wasm.push({path,sha256:createHash('sha256').update(body).digest('hex')});
 return route.fulfill({status:200,headers:{'content-security-policy':csp,'x-content-type-options':'nosniff'},contentType:({'.html':'text/html','.js':'text/javascript','.wasm':'application/wasm','.css':'text/css','.woff2':'font/woff2'})[extname(asset)]||'application/octet-stream',body});
 }catch{return route.fulfill({status:404,body:'missing fixture asset'});}
}
let body,status=200;
if(method==='GET'&&path==='/v1/auth/me')body={principal:'fixture-user',admin:false,csrf_token:'fixture-only'};
else if(method==='GET'&&path==='/v1/file-manager/disks')body={disks:[{slot_id:'P01',online:true,writable:true,available_bytes:'4096'}]};
else if(method==='GET'&&path==='/v1/file-manager/disks/P01/entries'){
 const dir=url.searchParams.get('path')||'';body={entries:dir==='a'?sources[0].entries:dir==='b'?[]:[{entry_id:'a',kind:'directory',name:'a',path:'a'},{entry_id:'b',kind:'directory',name:'b',path:'b'}],context:{repository_id:'fixture-repository',repository_path:dir,branch_id:'stable-namespace'},default_branch_scope:!dir,writable:true,directory_branches_supported:true,directory_file_count:'2',directory_branches:dir==='a'?(scopeMissing?[]:sources):dir==='b'?targets:[]};
}else if(method==='GET'&&path==='/v1/repositories/fixture-repository/directory-branches')body={repository_id:'fixture-repository',directory_branches:[...(scopeMissing?[]:sources),...targets]};
else if(method==='POST'&&path==='/v1/repositories/fixture-repository/directory-scopes'){
 const value=request.postDataJSON();assert.equal(value.path,'a');scopeMissing=false;scene.writes.push({method,path,body:value});body={operation_id:value.operation_id,state:'PUBLISHED',result:{directory_branch_id:'m'}};
}else if(method==='GET'&&path==='/v1/file-manager/files/f/history'){
 assert.equal(url.searchParams.get('repository_id'),'fixture-repository');assert.equal(url.searchParams.get('path'),'a/f.txt');body={file_id:'f',version_id:'F1',selected_branch_id:currentFileBranch,inherited:currentFileBranch==='file-f',materialized:currentFileBranch!=='file-f',note:'',versions:[{file_id:'f',version_id:'F1',created_at:'1',source_version_ids:[],note:'',content_state:'LIVE'}],branches:[{branch_id:'file-f',name:'main',version_id:'F1',current:currentFileBranch==='file-f'},{branch_id:'review',name:generatedBranchName,version_id:'F1',materialized:true,current:currentFileBranch==='review'}]};
}else if(method==='POST'&&path==='/v1/file-manager/files/f/select'){
 const value=request.postDataJSON();assert.equal(value.repository_id,'fixture-repository');assert.equal(value.path,'a/f.txt');assert.ok(['file-f','review'].includes(value.branch_id));currentFileBranch=value.branch_id;scene.writes.push({method,path,body:value});body={operation_id:value.operation_id,state:'PUBLISHED'};
}else if(method==='PATCH'&&path==='/v1/file-manager/files/f/branches/review'){
 const value=request.postDataJSON();assert.equal(value.name,'用户命名的方案');assert.ok(value.operation_id);assert.deepEqual(Object.keys(value).sort(),['name','operation_id']);generatedBranchName=value.name;scene.writes.push({method,path,body:value});body={operation_id:value.operation_id,state:'PUBLISHED',result:{file_id:'f',branch_id:'review',name:value.name}};
}else if(method==='POST'&&path==='/v1/file-manager/disks/P01/namespace-batch'){
 const value=request.postDataJSON();assert.ok(value.operation_id);assert.ok(value.items.length);scene.writes.push({method,path,body:value});
 if(pendingNext){pendingNext=false;pendingId=value.operation_id;body={operation_id:value.operation_id,state:'in_doubt'};}
 else if(conflictNext){conflictNext=false;status=409;body={code:'DIRECTORY_BRANCH_CONFLICT',message:'fixture frozen subtree conflict',retryable:false};}
 else body={operation_id:value.operation_id,state:'PUBLISHED',result:{action:value.kind}};
}else if(method==='GET'&&path===`/v1/operations/${pendingId}`){pollCount++;body={operation_id:pendingId,state:allowReceipt?'PUBLISHED':'in_doubt'};}
else{scene.unexpected.push({method,path});status=500;body={code:'UNEXPECTED_FIXTURE_API',message:'Every API is intercepted'};}
return route.fulfill({status,contentType:'application/json',body:JSON.stringify(body)});
});
const editor=page.getByRole('region',{name:language==='zh'?'批量文件操作':'Batch file operation',exact:true});
async function enter(){await page.goto(`http://zxs.fixture/?lang=${language}`);await page.locator('html[data-zxs-ready="true"]').waitFor();await page.locator('.zxs-disk-entry[data-slot="P01"]').click();await page.getByRole('button',{name:'a',exact:true}).click();await page.getByRole('checkbox',{name:language==='zh'?'选择 f.txt':'Select f.txt',exact:true}).waitFor();}
async function choose(combo,label){await combo.scrollIntoViewIfNeeded();await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));await combo.click();await page.getByRole('option',{name:label,exact:true}).click();}
async function start(kind){await page.locator('.zxs-file-toolbar').getByRole('button',{name:labels[kind],exact:true}).click();await editor.waitFor();await page.waitForFunction(()=>!document.querySelector('.zxs-batch-editor button')?.disabled);}
async function checkTwo(){await page.getByRole('checkbox',{name:language==='zh'?'选择 f.txt':'Select f.txt',exact:true}).check();await page.getByRole('checkbox',{name:language==='zh'?'选择 g.txt':'Select g.txt',exact:true}).check();}
try{
await enter();await checkTwo();assert.equal(await page.locator('.zxs-entry-page').count(),0);await start('move');await editor.getByRole('checkbox',{name:'n',exact:true}).check();
await editor.getByRole('textbox',{name:labels.target,exact:true}).fill('b');await editor.getByRole('textbox',{name:labels.target,exact:true}).press('Enter');
await choose(editor.getByRole('combobox',{name:labels.dest,exact:true}).nth(0),'l');await choose(editor.getByRole('combobox',{name:labels.dest,exact:true}).nth(1),'p');
assert.equal(await page.getByRole('dialog').count(),0);await page.screenshot({path:`${output}/move-review-${key}.png`,fullPage:true});
await editor.getByRole('button',{name:labels.apply,exact:true}).click();await editor.waitFor({state:'detached'});
let move=scene.writes.at(-1).body;assert.equal(move.kind,'move');assert.equal(move.items.length,4);assert.deepEqual(move.items.map(item=>[item.source.directory_branch_id,item.target.directory_branch_id]),[['m','l'],['m','l'],['n','p'],['n','p']]);
assert.equal(move.items[2].source.path,'a/alias.txt');assert.ok(move.items.every(item=>item.source.expected_tree_id===`tree-${item.source.directory_branch_id}`&&item.target.expected_tree_id===`tree-${item.target.directory_branch_id}`));
scene.checks.push('Two selected files and two explicit source/destination branch pairs submit four frozen scope items without Cartesian expansion or default-branch writes');
await checkTwo();await start('copy');await editor.getByRole('textbox',{name:labels.target,exact:true}).fill('b');await editor.getByRole('button',{name:labels.load,exact:true}).click();await choose(editor.getByRole('combobox',{name:labels.dest,exact:true}).first(),'l');
const before=scene.writes.length;await editor.getByRole('button',{name:labels.cancel,exact:true}).click();assert.equal(scene.writes.length,before);scene.checks.push('Destination review and cancellation only read scopes and never mutate a directory or SMB default');
await start('copy');await editor.getByRole('textbox',{name:labels.target,exact:true}).fill('b');await editor.getByRole('textbox',{name:labels.target,exact:true}).press('Enter');await choose(editor.getByRole('combobox',{name:labels.dest,exact:true}).first(),'l');await editor.getByRole('button',{name:language==='zh'?'添加分支映射':'Add branch mapping',exact:true}).click();await choose(editor.getByRole('combobox',{name:labels.dest,exact:true}).nth(1),'p');await editor.getByRole('button',{name:labels.apply,exact:true}).click();await editor.waitFor({state:'detached'});const copied=scene.writes.at(-1).body;assert.equal(copied.kind,'copy');assert.equal(copied.items.length,4);assert.ok(copied.items.every(item=>item.source.directory_branch_id==='m'));assert.deepEqual([...new Set(copied.items.map(item=>item.target.directory_branch_id))],['l','p']);scene.checks.push('One source branch can copy selected files into two explicitly added destination mappings without changing source selection');
await checkTwo();

await start('rename');await editor.getByRole('checkbox',{name:'n',exact:true}).check();const input=editor.getByRole('textbox',{name:labels.name,exact:true}).first();await input.fill('客户报告.txt');await input.focus();await input.press('End');await input.press('!');assert.equal(await input.inputValue(),'客户报告.txt!');assert.ok(await input.evaluate(node=>document.activeElement===node));await input.fill('客户报告.txt');
conflictNext=true;await editor.getByRole('button',{name:labels.apply,exact:true}).click();await editor.getByRole('status').filter({hasText:'fixture frozen subtree conflict'}).waitFor();assert.equal(await input.inputValue(),'客户报告.txt');assert.ok(await editor.isVisible());assert.equal(await page.getByRole('dialog').count(),0);
scene.checks.push('Underline name editors retain focus during typing; a 409 keeps names, branch selection and the review panel intact');
await page.screenshot({path:`${output}/conflict-review-${key}.png`,fullPage:true});await editor.getByRole('button',{name:labels.cancel,exact:true}).click();
await start('del');await editor.getByRole('checkbox',{name:'n',exact:true}).check();assert.equal(await editor.getByRole('textbox',{name:labels.name,exact:true}).count(),0);await editor.getByRole('button',{name:labels.apply,exact:true}).click();await editor.waitFor({state:'detached'});const deleted=scene.writes.at(-1).body;assert.equal(deleted.kind,'delete');assert.equal(deleted.items.length,4);assert.ok(deleted.items.every(item=>!('target'in item)&&['m','n'].includes(item.source.directory_branch_id)));
scene.checks.push('Delete uses the inline selected-branch review, emits no targets, and opens no dialog');
scopeMissing=true;await enter();await page.getByRole('checkbox',{name:labels.all,exact:true}).check();await start('del');await editor.getByRole('button',{name:labels.init,exact:true}).click();await editor.getByRole('checkbox',{name:'m',exact:true}).waitFor();assert.equal(scene.writes.at(-1).path,'/v1/repositories/fixture-repository/directory-scopes');await editor.getByRole('button',{name:labels.apply,exact:true}).click();await editor.waitFor({state:'detached'});assert.ok(scene.writes.at(-1).body.items.some(item=>item.source.entry_id==='folder'));
scene.checks.push('A legacy namespace has an explicit metadata-only scope initialization action; GET requests never create its branch records');
await checkTwo();await start('copy');await editor.getByRole('textbox',{name:labels.target,exact:true}).fill('b');await editor.getByRole('textbox',{name:labels.target,exact:true}).press('Enter');await choose(editor.getByRole('combobox',{name:labels.dest,exact:true}).first(),'l');pendingNext=true;await editor.getByRole('button',{name:labels.apply,exact:true}).click();await page.waitForFunction(()=>document.querySelector('.zxs-batch-editor button:last-child')?.disabled||document.querySelector('.zxs-batch-editor .zxs-file-toolbar:last-child button')?.disabled);assert.ok(await editor.isVisible());assert.ok(await editor.getByRole('button',{name:labels.apply,exact:true}).isDisabled());const count=scene.writes.length;await new Promise(resolve=>setTimeout(resolve,2300));assert.ok(pollCount>=1);assert.equal(scene.writes.length,count);allowReceipt=true;await editor.waitFor({state:'detached'});assert.equal(scene.writes.length,count);scene.checks.push('An uncertain batch retains its review and original operation ID, blocks repeat submission, and closes only after that operation is confirmed by polling');

await page.getByRole('button',{name:'f.txt',exact:true}).click();const fileBranch=page.getByRole('combobox',{name:language==='zh'?'当前分支':'Current branch',exact:true});await fileBranch.waitFor();await choose(fileBranch,language==='zh'?'主版本':'Main');await page.waitForFunction(()=>!document.querySelector('.zxs-file-branches [role="combobox"]')?.disabled);assert.equal(scene.writes.at(-1).body.branch_id,'file-f');await choose(fileBranch,'方案甲-7c2a');await page.waitForFunction(()=>!document.querySelector('.zxs-file-branches [role="combobox"]')?.disabled);assert.equal(scene.writes.at(-1).path,'/v1/file-manager/files/f/select');scene.checks.push('File history and file-branch selection carry the logical repository/path context; reselecting the displayed main branch still submits explicit write intent');
const renameLabel=language==='zh'?'重命名文件分支':'Rename file branch',rename=page.getByRole('button',{name:renameLabel,exact:true});await rename.waitFor();const treeBefore=await page.locator('.zxs-version-node').evaluateAll(nodes=>nodes.map(node=>node.dataset.versionId));let beforeRename=scene.writes.length;
await rename.click();let branchName=page.getByRole('textbox',{name:renameLabel,exact:true});assert.equal(await branchName.inputValue(),'方案甲-7c2a');await branchName.fill('');await branchName.press('Enter');assert.equal(scene.writes.length,beforeRename);
await rename.click();branchName=page.getByRole('textbox',{name:renameLabel,exact:true});await branchName.fill('a'.repeat(129));await branchName.press('Enter');assert.equal(await branchName.getAttribute('aria-invalid'),'true');assert.equal(scene.writes.length,beforeRename);await branchName.fill('用户命名的方案');await branchName.press('Enter');await page.waitForFunction(()=>document.querySelector('.zxs-file-branches [role="combobox"]')?.textContent?.includes('用户命名的方案'));assert.equal(scene.writes.length,beforeRename+1);assert.equal(scene.writes.at(-1).path,'/v1/file-manager/files/f/branches/review');assert.deepEqual(await page.locator('.zxs-version-node').evaluateAll(nodes=>nodes.map(node=>node.dataset.versionId)),treeBefore);
await rename.click();branchName=page.getByRole('textbox',{name:renameLabel,exact:true});await branchName.fill('cancelled-name');await branchName.press('Escape');assert.equal(scene.writes.length,beforeRename+1);assert.equal(await page.getByRole('dialog').count(),0);await page.screenshot({path:`${output}/file-branch-rename-${key}.png`,fullPage:true});scene.checks.push('Auto-created public file branches use the shared 128-byte underline editor: blank/Escape cancel, oversized UTF-8 names reject, and PATCH renames the stable branch without new version nodes');

assert.deepEqual(scene.unexpected,[]);assert.deepEqual(scene.pageErrors,[]);assert.ok(scene.wasm.length);scene.passed=true;
}catch(error){scene.passed=false;scene.error=String(error.stack||error);await page.screenshot({path:`${output}/failure-${key}.png`,fullPage:true}).catch(()=>{});}
await context.close();scenes.push(scene);await writeFile(`${output}/result-${key}.json`,JSON.stringify(scene,null,2)+'\n');console.log(`${key}: ${scene.passed?'PASS':'FAIL'} (${scene.checks.length} behavior groups)`);
}
}finally{await browser.close();}
await writeFile(`${output}/browser-result.json`,JSON.stringify({passed:scenes.every(scene=>scene.passed),dist,scenes},null,2)+'\n');assert.ok(scenes.every(scene=>scene.passed),scenes.filter(scene=>!scene.passed).map(scene=>scene.error).join('\n'));
