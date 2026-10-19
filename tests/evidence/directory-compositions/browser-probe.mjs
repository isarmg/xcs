// Directory composition controls in actual WASM; every API is intercepted.
import assert from 'node:assert/strict';
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
const {chromium}=await import(process.env.XCS_PLAYWRIGHT_MODULE||'playwright');
const root=resolve(process.env.XCS_PRODUCT_ROOT||fileURLToPath(new URL('../../../',import.meta.url)));
const base=process.env.XCS_PREVIEW_URL||'http://127.0.0.1:17401';
const output=resolve(process.env.XCS_DIRECTORY_EVIDENCE||`${root}/tests/evidence/directory-compositions`);
const browser=await chromium.launch({headless:true,args:['--no-sandbox']});
const scenes=[];await mkdir(output,{recursive:true});
try{
  for(const [language,width,theme]of[['zh',1440,'light'],['en',390,'dark']]){
    const key=`${language}-${theme}-${width}`;
    const labels=language==='zh'?{main:'当前主分支（SMB 默认）',folder:'当前目录分支',rename:'重命名分支',create:'创建目录分支',name:'分支名称',search:'搜索当前文件夹',initial:'主版本',history:'历史记录',audit:['登记目录分支','重命名目录分支','切换 SMB 默认目录分支','切换目录分支','写入目录子树']}:{main:'Current main branch (SMB default)',folder:'Current folder branch',rename:'Rename branch',create:'Create folder branch',name:'Branch name',search:'Search this folder',initial:'Main',history:'History',audit:['Registered folder branch','Renamed folder branch','Switched SMB default folder branch','Switched folder branch','Wrote folder subtree']};
    const state={key,checks:[],unexpected:[],pageErrors:[],wasm:[],writes:[]};
    const context=await browser.newContext({viewport:{width,height:1050},colorScheme:theme}),page=await context.newPage(),pending=[];
    page.setDefaultTimeout(20000);
    let rootBranches=[],rootCurrent='auto-main',childCurrent='child-main';
    const children=[{directory_branch_id:'child-main',name:'auto-child-0001',materialized:true},{directory_branch_id:'child-draft',name:'child-draft',materialized:false}];
    await page.addInitScript(theme=>localStorage.setItem('xcs-theme',theme),theme);
    page.on('pageerror',error=>state.pageErrors.push(error.message));
    page.on('response',response=>{const path=new URL(response.url()).pathname;if(path.endsWith('.wasm'))pending.push((async()=>{const body=await response.body();state.wasm.push({path,sha256:createHash('sha256').update(body).digest('hex'),matchesCandidate:body.equals(await readFile(`${root}/web/dist${path}`))});})());});
    await page.route('**/v1/**',async route=>{
      const request=route.request(),url=new URL(request.url()),path=url.pathname,method=request.method();
      const publish=(body,result={})=>({operation_id:body.operation_id,state:'PUBLISHED',result});
      let body;
      if(method==='GET'&&path==='/v1/auth/me')body={principal:'fixture-user',admin:false,csrf_token:'fixture-only'};
      else if(method==='GET'&&path==='/v1/file-manager/disks')body={disks:[{slot_id:'P01',online:true,writable:true,available_bytes:'4096'}]};
      else if(method==='GET'&&path==='/v1/file-manager/history')body={events:['directory.branches.create','directory.branch.rename','directory.default.select','directory.branches.select','directory.tree.write'].map((action,index)=>({event_id:`event-${index}`,action,file_id:'',version_id:null,directory_branch_id:`directory-${index}`,head_tree_id:`subtree-${index}`,parent_tree_id:index===4?'parent-subtree':null,slot_id:'P01',path:index===3?'folder':'',timestamp:String(index+1),principal:'fixture-user',message:'directory composition'}))};
      else if(method==='GET'&&path==='/v1/file-manager/disks/P01/entries'){
        const child=url.searchParams.get('path')==='folder';
        body={entries:child?[]:[{entry_id:'folder-entry',kind:'directory',name:'folder',path:'folder'}],context:{repository_id:'fixture-repository',branch_id:'stable-namespace'},default_branch_scope:!child,writable:true,directory_file_count:'0',directory_branches_supported:true,directory_branches:(child?children:rootBranches).map(branch=>({...branch,current:branch.directory_branch_id===(child?childCurrent:rootCurrent)}))};
      }else if(method==='POST'&&path==='/v1/file-manager/disks/P01/directory-branches'){
        const value=request.postDataJSON();state.writes.push({method,path,body:value});
        assert.equal(value.path,'');assert.equal(value.name,'empty-draft');
        rootBranches=[{directory_branch_id:'auto-main',name:'auto-main-0001',materialized:true},{directory_branch_id:'root-draft',name:value.name,materialized:false}];
        body=publish(value,{directory_branch_id:'root-draft',count:'0',copied_files:'0',placeholder_files:'0'});
      }else if(method==='PATCH'&&path==='/v1/repositories/fixture-repository/directory-branches/auto-main'){
        const value=request.postDataJSON();state.writes.push({method,path,body:value});
        assert.equal(value.name,'renamed-main');rootBranches[0].name=value.name;
        body=publish(value,{directory_branch_id:'auto-main',name:value.name});
      }else if(method==='PUT'&&path==='/v1/repositories/fixture-repository/default-branch'){
        const value=request.postDataJSON();state.writes.push({method,path,body:value});
        assert.equal(value.directory_branch_id,'root-draft');rootCurrent=value.directory_branch_id;
        body=publish(value,{directory_branch_id:rootCurrent,branch_id:'stable-namespace'});
      }else if(method==='POST'&&path==='/v1/file-manager/disks/P01/directory-select'){
        const value=request.postDataJSON();state.writes.push({method,path,body:value});
        assert.equal(value.path,'folder');assert.equal(value.name,'child-draft');childCurrent='child-draft';
        body=publish(value,{directory_branch_id:childCurrent});
      }else{state.unexpected.push({method,path});body={code:'UNEXPECTED_FIXTURE_API',message:'Every API request is intercepted'};}
      return route.fulfill({status:state.unexpected.length?500:200,contentType:'application/json',body:JSON.stringify(body)});
    });
    try{
      await page.goto(`${base}/?lang=${language}`);
      await page.locator('html[data-xcs-ready="true"]').waitFor();
      await page.locator('.xcs-disk-entry[data-slot="P01"]').click();
      const rootSelect=page.getByRole('combobox',{name:labels.main,exact:true});
      await rootSelect.waitFor();assert.ok(await rootSelect.isDisabled());assert.equal(await rootSelect.innerText(),labels.initial);
      assert.equal(await page.getByRole('button',{name:labels.rename,exact:true}).count(),0);
      assert.ok(await page.getByRole('button',{name:labels.create,exact:true}).isEnabled());
      state.checks.push('Initial namespace main has no invented null branch option or rename target; an empty directory can register a composition');
      await page.getByRole('button',{name:labels.create,exact:true}).click();
      await page.getByRole('textbox',{name:labels.name,exact:true}).fill('empty-draft');
      await page.getByRole('textbox',{name:labels.name,exact:true}).press('Enter');
      await page.waitForFunction(()=>document.querySelector('.xcs-file-toolbar [role="combobox"]')?.textContent?.includes('auto-main-0001'));
      const rename=page.getByRole('button',{name:labels.rename,exact:true});
      await rename.click();
      let input=page.getByRole('textbox',{name:labels.rename,exact:true});assert.equal(await input.inputValue(),'auto-main-0001');
      await input.fill('');await input.press('Enter');assert.equal(state.writes.length,1);
      await rename.click();input=page.getByRole('textbox',{name:labels.rename,exact:true});await input.fill('cancelled-name');await page.getByRole('searchbox',{name:labels.search,exact:true}).focus();assert.equal(state.writes.length,1);
      await rename.click();input=page.getByRole('textbox',{name:labels.rename,exact:true});await input.fill('a'.repeat(129));await input.press('Enter');assert.equal(await input.getAttribute('aria-invalid'),'true');assert.equal(state.writes.length,1);
      await input.fill('renamed-main');await input.press('Enter');
      await page.waitForFunction(()=>document.querySelector('.xcs-file-toolbar [role="combobox"]')?.textContent?.includes('renamed-main'));
      assert.equal(state.writes.length,2);
      state.checks.push('Generated composition name uses the shared underline editor: empty and blur cancel, 129 bytes reject, Enter renames the captured record ID');
      await rootSelect.click();await page.getByRole('option',{name:'empty-draft',exact:true}).click();
      await page.waitForFunction(()=>document.querySelector('.xcs-file-toolbar [role="combobox"]')?.textContent?.includes('empty-draft'));
      assert.equal(state.writes[2].method,'PUT');
      assert.equal(state.writes[2].body.directory_branch_id,'root-draft');
      state.checks.push('Root main selector selects the SMB default by real directory composition ID, without rebinding the stable namespace');
      await page.screenshot({path:`${output}/root-${key}.png`,fullPage:true});
      await page.getByRole('button',{name:'folder',exact:true}).click();
      const childSelect=page.getByRole('combobox',{name:labels.folder,exact:true});await childSelect.waitFor();await childSelect.click();await page.getByRole('option',{name:'child-draft',exact:true}).click();
      await page.waitForFunction(()=>document.querySelector('.xcs-file-toolbar [role="combobox"]')?.textContent?.includes('child-draft'));
      assert.equal(state.writes[3].method,'POST');assert.equal(state.writes[3].body.name,'child-draft');
      assert.ok(await page.getByRole('button',{name:labels.rename,exact:true}).isEnabled());
      state.checks.push('Subfolder selector restores a saved combination by name and keeps its own rename target; no private file branch creation is requested');
      await page.screenshot({path:`${output}/subfolder-${key}.png`,fullPage:true});
      await page.getByRole('button',{name:labels.history,exact:true}).click();
      await page.locator('.xcs-history-row').first().waitFor();
      assert.deepEqual(await page.locator('.xcs-history-action').allInnerTexts(),labels.audit);
      await page.locator('.xcs-history-row').first().click();
      assert.ok(await page.locator('.xcs-history-page').isVisible());
      assert.equal(await page.locator('.xcs-entry-page').count(),0);
      state.checks.push('Directory audit actions have localized labels and empty file IDs never navigate to file details');
      await page.screenshot({path:`${output}/audit-${key}.png`,fullPage:true});
      await Promise.all(pending);assert.ok(state.wasm.length&&state.wasm.every(item=>item.matchesCandidate));assert.deepEqual(state.unexpected,[]);assert.deepEqual(state.pageErrors,[]);state.passed=true;
    }catch(error){state.passed=false;state.error=String(error.stack||error);await page.screenshot({path:`${output}/failure-${key}.png`,fullPage:true}).catch(()=>{});}
    await context.close();scenes.push(state);await writeFile(`${output}/result-${key}.json`,JSON.stringify(state,null,2)+'\n');console.log(`${key}: ${state.passed?'PASS':'FAIL'} (${state.checks.length} behavior groups)`);
  }
}finally{await browser.close();}
await writeFile(`${output}/browser-result.json`,JSON.stringify({passed:scenes.every(scene=>scene.passed),scenes},null,2)+'\n');
assert.ok(scenes.every(scene=>scene.passed),scenes.filter(scene=>!scene.passed).map(scene=>scene.error).join('\n'));
