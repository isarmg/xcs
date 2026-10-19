// Read-only acceptance of the actual local preview, without credentials.
import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {resolve} from 'node:path';
const {chromium}=await import(process.env.XCS_PLAYWRIGHT_MODULE||'playwright');
const root=resolve(new URL('../../../',import.meta.url).pathname);
const evidence=resolve(root,'tests/evidence/merge-tags-backup');
const receipt=JSON.parse(await readFile(resolve(evidence,'candidate-receipt.json'),'utf8'));
const expected=receipt.artifacts.find(item=>item.path==='web/dist/pkg/xcs_bg.wasm').sha256;
const browser=await chromium.launch({headless:true,args:['--no-sandbox']});
try {
  const context=await browser.newContext({viewport:{width:1440,height:1000}});
  const page=await context.newPage();
  const errors=[],blocked=[];
  page.on('pageerror',error=>errors.push(error.message));
  await page.route('**/*',route=>{
    const method=route.request().method();
    if(!['GET','HEAD'].includes(method)){
      blocked.push({method,path:new URL(route.request().url()).pathname});
      return route.abort();
    }
    return route.continue();
  });
  const wasmResponse=page.waitForResponse(response=>new URL(response.url()).pathname==='/pkg/xcs_bg.wasm');
  const response=await page.goto('http://127.0.0.1:7400/');
  assert.equal(response.status(),200);
  await page.waitForFunction(()=>document.documentElement.getAttribute('data-xcs-ready')==='true');
  await page.locator('input[type="password"]').waitFor({state:'visible'});
  const wasm=await wasmResponse;
  assert.equal(wasm.status(),200);
  assert.match(wasm.headers()['content-type'],/^application\/wasm/);
  assert.equal(createHash('sha256').update(await wasm.body()).digest('hex'),expected);
  assert.equal(await page.locator('html').getAttribute('data-xcs-runtime'),'xcs-web');
  assert.deepEqual(errors,[]);
  const csp=response.headers()['content-security-policy'];
  assert.ok(csp.includes("style-src 'self'")&&!csp.includes("'unsafe-inline'"));
  await page.screenshot({path:resolve(evidence,'served-login.png'),fullPage:true});
  const result={passed:true,preview_http_status:200,actual_wasm_sha256:expected,strict_csp:true,runtime:'xcs-web',ready:true,login_rendered:true,page_errors:errors,blocked_mutations:blocked,scope:'Unauthenticated actual preview GETs only; no credentials or business writes.'};
  await writeFile(resolve(evidence,'served-browser.json'),JSON.stringify(result,null,2)+'\n');
  console.log('PASS: actual preview renders login with the exact final WASM, runtime markers and strict CSP');
} finally {await browser.close();}
