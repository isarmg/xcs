// Test-only JavaScript; all shipped interaction and simulation logic is Rust.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {resolve} from 'node:path';
const {chromium}=await import(process.env.XCS_PLAYWRIGHT_MODULE||'playwright');
const demo=resolve(fileURLToPath(new URL('../',import.meta.url)));
const root=resolve(demo,'../..');
const evidence=resolve(demo,'tests/evidence');await mkdir(evidence,{recursive:true});
const base=process.env.XCS_DEMO_URL||'http://127.0.0.1:7400/demo/';
const manifest=JSON.parse(await readFile(resolve(demo,'dist/build-manifest.json'),'utf8'));
const hash=data=>createHash('sha256').update(data).digest('hex');
for(const [path,expected]of Object.entries(manifest.inputs))assert.equal(hash(await readFile(resolve(root,path))),expected,`source receipt: ${path}`);
const errors=[],business=[];const result={passed:false,base,scenarios:[],free_operations:[],viewports:[],receipt_inputs:Object.keys(manifest.inputs).length};
const browser=await chromium.launch({headless:true,args:['--no-sandbox']});
try {
  const context=await browser.newContext({viewport:{width:1440,height:1100}});
  const page=await context.newPage();
  page.on('pageerror',e=>errors.push(e.message));
  await page.route('**/*',route=>{
    const req=route.request();if(new URL(req.url()).pathname.startsWith('/v1')||!['GET','HEAD'].includes(req.method())){business.push({method:req.method(),url:req.url()});return route.abort();}
    return route.continue();
  });
  const wasmPromise=page.waitForResponse(r=>new URL(r.url()).pathname.endsWith('/pkg/xcs_bg.wasm'));
  const response=await page.goto(base);assert.equal(response.status(),200);
  assert.ok(response.headers()['content-security-policy'].includes("style-src 'self'"));
  await page.waitForFunction(()=>document.documentElement.getAttribute('data-xcs-ready')==='true');
  const wasm=await wasmPromise;assert.match(wasm.headers()['content-type'],/^application\/wasm/);
  assert.equal(hash(await wasm.body()),manifest.files['pkg/xcs_bg.wasm']);result.served_wasm_sha256=manifest.files['pkg/xcs_bg.wasm'];
  assert.equal(await page.locator('html').getAttribute('data-xcs-runtime'),'xcs-web');
  async function choose(label,value){await page.getByRole('combobox',{name:label,exact:true}).click();await page.getByRole('option',{name:value,exact:typeof value==='string'}).click();}
  async function count(which){return Number((await page.locator(`[data-demo-count="${which}"]`).innerText()).match(/\d+/)[0]);}
  async function graphCheck(){
    const graphs=await page.locator('.xcs-demo-graph').evaluateAll(svgs=>svgs.map(svg=>{
      const nodes=new Map([...svg.querySelectorAll('foreignObject')].map(f=>[f.querySelector('[data-demo-node]').dataset.demoNode,{y:Number(f.getAttribute('y')),h:Number(f.getAttribute('height'))}]));
      return [...svg.querySelectorAll('[data-parent]')].map(p=>({parent:nodes.get(p.dataset.parent),child:nodes.get(p.dataset.child),d:p.getAttribute('d')}));
    }));
    for(const edges of graphs)for(const e of edges){assert.ok(e.parent.y>=e.child.y+e.child.h+18);assert.ok(!/[CLQS]/.test(e.d),'orthogonal lines and round arcs only');}
    assert.equal(await page.locator('.xcs-demo-node').evaluateAll(nodes=>nodes.every(n=>n.querySelectorAll(':scope > .xcs-row').length===3)),true);
  }
  const scenes=[
    ['文件分支：登记 → 切换 → 首写',4,3],['目录分支：按文件逐个首写',5,4],['子目录分支与 SMB 默认组合',5,3],['改名、移动、复制与删除',6,3],
    ['合并文件分支：复用现有头节点',5,4],['合并文件分支：选择历史节点',5,4],['标签继承与历史节点旁支',3,3],['副本份数与各盘独立前缀',3,3]
  ];
  for(const [title,steps,bodies]of scenes){
    await choose('演示过程',title);assert.equal(await count('bodies'),2);
    for(let n=1;n<=steps;n++){await page.getByRole('button',{name:'下一步',exact:true}).click();await page.waitForFunction(n=>document.querySelector('[data-demo-step="description"]').textContent.trim().startsWith(`${n} /`),n);await graphCheck();}
    assert.equal(await count('bodies'),bodies,title);
    await page.getByRole('button',{name:'上一步',exact:true}).click();assert.match(await page.locator('[data-demo-step="description"]').innerText(),new RegExp(`^${steps-1} /`));
    await page.getByRole('button',{name:'下一步',exact:true}).click();result.scenarios.push({title,steps,bodies,passed:true});
    if(title.startsWith('合并文件分支：选择历史'))await page.screenshot({path:resolve(evidence,'process-merge.png'),fullPage:true});
  }
  await choose('演示过程',scenes[0][0]);await page.getByRole('button',{name:'播放',exact:true}).click();
  await page.waitForFunction(()=>document.querySelector('[data-demo-step="description"]').textContent.trim().startsWith('1 /'));
  await page.getByRole('button',{name:'暂停',exact:true}).click();const paused=await page.locator('[data-demo-step="description"]').innerText();await page.waitForTimeout(2600);assert.equal(await page.locator('[data-demo-step="description"]').innerText(),paused);
  await page.getByRole('button',{name:'下一步',exact:true}).click();
  await page.getByRole('link',{name:'从这一步自行操作 →'}).click();await page.waitForFunction(()=>document.documentElement.getAttribute('data-xcs-ready')==='true');
  assert.match(page.url(),/playground.html\?scenario=file&step=2/);assert.equal(await count('bodies'),2);assert.match(await page.locator('[data-demo-path="A/报告.txt"]').innerText(),/审阅/);
  await page.getByRole('button',{name:'重置演示',exact:true}).click();
  const input=()=>page.locator('[data-xcs-demo-input="value"]');
  const submit=()=>page.getByRole('button',{name:'执行操作',exact:true}).click();
  async function op(value){await choose('操作',value);}
  await op('登记文件分支');await input().fill('审阅');await submit();assert.equal(await count('bodies'),2);assert.equal(await count('files'),2);result.free_operations.push('register without body');
  await op('切换文件分支');await choose('目标分支','审阅（待首写）');await submit();assert.equal(await count('bodies'),2);
  await op('写入文件');await input().fill('预算 200');await submit();assert.equal(await count('bodies'),3);assert.match(await page.locator('[data-demo-path="A/报告.txt"]').innerText(),/预算 200/);result.free_operations.push('switch + first write');
  await op('编辑节点备注');await input().fill('中'.repeat(43));await submit();assert.match(await page.getByRole('alert').innerText(),/128/);await input().fill('已核对');await submit();assert.equal(await count('bodies'),3);assert.match(await page.locator('[data-demo-node="selected-detail"]').innerText(),/已核对/);result.free_operations.push('UTF-8 note limit');
  await op('修改节点标签');await input().fill('已审阅, 可发布');await submit();assert.equal(await count('bodies'),3);assert.match(await page.locator('[data-demo-node="selected-detail"]').innerText(),/可发布/);result.free_operations.push('tag metadata reuse');
  await op('合并文件分支');await page.getByRole('checkbox',{name:'main',exact:true}).check();await page.getByRole('checkbox',{name:'审阅',exact:true}).check();await choose('合并后归属分支','main');const beforeMerge=await count('files');await page.getByRole('button',{name:'确认合并并执行',exact:true}).click();assert.equal(await count('files'),beforeMerge);assert.match(await page.getByRole('status').last().innerText(),/复用/);result.free_operations.push('explicit result and owner, head reused');
  await op('切换文件分支');await choose('目标分支','审阅（只读别名）');await submit();await op('写入文件');await input().fill('拒绝写入');await submit();assert.match(await page.getByRole('alert').innerText(),/只读/);assert.equal(await count('bodies'),3);result.free_operations.push('merged alias read-only');
  await page.getByRole('button',{name:'重置演示',exact:true}).click();
  await page.locator('[data-demo-path=""]').click();await op('登记目录分支');await input().fill('archive');await submit();
  await page.locator('[data-demo-path="A"]').click();await op('重命名');await input().fill('项目');await submit();assert.equal(await count('bodies'),2);assert.equal(await count('files'),4);result.free_operations.push('folder rename path nodes');
  await page.locator('[data-demo-path="项目/报告.txt"]').click();await op('移动');await page.getByRole('textbox',{name:'目标目录',exact:true}).fill('B');await submit();assert.equal(await page.locator('[data-demo-path="项目/报告.txt"]').count(),0);assert.equal(await page.locator('[data-demo-path="B/报告.txt"]').count(),1);result.free_operations.push('move membership');
  await page.locator('[data-demo-path="B/报告.txt"]').click();await op('复制');await page.getByRole('textbox',{name:'目标目录',exact:true}).fill('项目');await submit();assert.equal(await count('bodies'),3);result.free_operations.push('independent copy');
  await page.locator('[data-demo-path="项目/报告.txt"]').click();await op('删除引用');await submit();assert.equal(await page.locator('[data-demo-path="项目/报告.txt"]').count(),0);
  await page.getByRole('button',{name:'撤销',exact:true}).click();assert.equal(await page.locator('[data-demo-path="项目/报告.txt"]').count(),1);await page.getByRole('button',{name:'重做',exact:true}).click();assert.equal(await page.locator('[data-demo-path="项目/报告.txt"]').count(),0);result.free_operations.push('delete / undo / redo');
  await choose('当前根目录分支','archive');assert.equal(await page.locator('[data-demo-path="A/报告.txt"]').count(),1);assert.equal(await page.locator('[data-demo-path="B/报告.txt"]').count(),0);result.free_operations.push('unselected composition frozen');
  await page.locator('[data-demo-path="A"]').click();await op('设置备份前缀');await input().fill('档案/项目 | 异地/项目');await submit();assert.match(await page.locator('main').innerText(),/B01 → 档案\/项目/);result.free_operations.push('user backup prefixes');
  await op('设置 SMB 默认组合');await choose('SMB 默认根目录分支','archive');await submit();assert.match(await page.locator('main').innerText(),/SMB 默认 archive/);result.free_operations.push('SMB default composition');
  await page.screenshot({path:resolve(evidence,'playground-desktop.png'),fullPage:true});
  await page.locator('[data-demo-path="A/报告.txt"]').click();await op('写入文件');await input().fill('<img src=x onerror=alert(1)>');await submit();assert.equal(await page.locator('img').count(),0);assert.ok((await page.locator('main').innerText()).includes('<img src=x onerror=alert(1)>'));result.free_operations.push('escaped user content');
  for(const [width,height]of [[1440,1000],[800,1000],[390,844]]){
    await page.setViewportSize({width,height});await page.waitForTimeout(150);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),true,`page overflow ${width}`);
    const canvases=await page.locator('.xcs-demo-canvas').evaluateAll(es=>es.map(e=>({width:e.clientWidth,scroll:e.scrollWidth})));assert.ok(canvases.every(c=>c.width<=width));
    await page.screenshot({path:resolve(evidence,`playground-${width}.png`),fullPage:true});result.viewports.push({width,height,internal_tree_scroll:true});
  }
  await page.getByRole('link',{name:'静态图示',exact:true}).click();assert.equal(await page.locator('.cases label').count(),27);assert.equal(await page.locator('.panel:visible').count(),1);
  for(let i=0;i<27;i++){await page.locator(`label[for="case-${i}"]`).click();assert.equal(await page.locator(`.panel[data-case="${i}"]`).isVisible(),true);assert.equal(await page.locator('.panel:visible').count(),1);}
  assert.deepEqual(business,[]);assert.deepEqual(errors,[]);result.passed=true;result.business_requests=business;result.page_errors=errors;result.static_reference_cases=27;
} catch(error){result.error=String(error.stack||error);throw error;} finally {await writeFile(resolve(evidence,'browser-result.json'),JSON.stringify(result,null,2)+'\n');await browser.close();}
console.log(`PASS: ${result.scenarios.length} guided processes, ${result.free_operations.length} free operation checks, ${result.viewports.length} viewports, 27 references`);
