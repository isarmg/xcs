import {createServer} from 'node:http';
import {readFile,writeFile} from 'node:fs/promises';
import {resolve,extname,sep} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';

const {chromium}=await import(process.env.XCS_PLAYWRIGHT_MODULE||'playwright');
const root=resolve(process.env.XCS_PRODUCT_ROOT||fileURLToPath(new URL('../../../',import.meta.url)));
const fixture=resolve(process.env.XCS_ANIMATED_FIXTURE||'/tmp/xcs-animated-show-fixture/dist');
const result={passed:false,independentFixture:true,productionRoutesUnchanged:true,pageErrors:[],consoleErrors:[],resources:[]};
const mime={'.html':'text/html; charset=utf-8','.js':'text/javascript','.wasm':'application/wasm'};
const server=createServer(async(request,response)=>{
  try{
    const relative=decodeURIComponent(new URL(request.url,'http://fixture').pathname);
    if(relative==='/favicon.ico'){response.writeHead(204);response.end();return;}
    const file=resolve(fixture,'.'+(relative==='/'?'/index.html':relative));
    if(!file.startsWith(fixture+sep)){response.writeHead(400);response.end();return;}
    const body=await readFile(file);response.writeHead(200,{'Content-Type':mime[extname(file)]||'application/octet-stream','Cache-Control':'no-store'});response.end(body);
  }catch{response.writeHead(404);response.end();}
});
await new Promise((done,reject)=>{server.once('error',reject);server.listen(0,'127.0.0.1',done)});
const browser=await chromium.launch({headless:true,args:['--no-sandbox']});
const page=await browser.newPage({viewport:{width:800,height:600}});
page.on('pageerror',error=>result.pageErrors.push(error.message));
page.on('console',message=>{if(message.type()==='error')result.consoleErrors.push(message.text())});
const resources=[];
page.on('response',response=>{if(response.url().endsWith('.wasm'))resources.push((async()=>{const body=await response.body();result.resources.push({name:'pkg/fixture_bg.wasm',status:response.status(),mime:response.headers()['content-type'],sha256:createHash('sha256').update(body).digest('hex')})})())});
try{
  await page.goto(`http://127.0.0.1:${server.address().port}/index.html`);
  await page.waitForFunction(()=>document.documentElement.dataset.ready==='true');
  await page.locator('#animated-child').waitFor();
  result.repeatedFalseThenShow=await page.evaluate(async()=>{
    const events=[],start=performance.now(),click=id=>{document.getElementById(id).click();events.push({id,at:performance.now()-start})},wait=ms=>new Promise(done=>setTimeout(done,ms));
    click('hide');await wait(50);click('repeat');await wait(50);click('show');await wait(250);
    return{events,elapsed:performance.now()-start,childPresent:!!document.getElementById('animated-child')};
  });
  assert.ok(result.repeatedFalseThenShow.events.at(-1).at<200,'show must happen before the original hide timer');
  assert.ok(result.repeatedFalseThenShow.childPresent,'an overwritten old timer must not hide the newly shown child');
  result.repeatedFalseThenUnmount=await page.evaluate(async()=>{
    const start=performance.now(),wait=ms=>new Promise(done=>setTimeout(done,ms));
    document.getElementById('hide').click();await wait(50);document.getElementById('repeat').click();await new Promise(done=>requestAnimationFrame(done));window.xcsFixture.unmount();await wait(250);
    const element=document.getElementById('fixture');return{elapsed:performance.now()-start,childElementCount:element.childElementCount,text:element.textContent.trim(),childPresent:!!document.getElementById('animated-child')};
  });
  assert.equal(result.repeatedFalseThenUnmount.childElementCount,0);assert.equal(result.repeatedFalseThenUnmount.text,'');assert.equal(result.repeatedFalseThenUnmount.childPresent,false);
  await Promise.all(resources);assert.equal(result.resources.length,1);assert.equal(result.resources[0].status,200);assert.equal(result.resources[0].mime,'application/wasm');assert.deepEqual(result.pageErrors,[]);assert.deepEqual(result.consoleErrors,[]);result.passed=true;
}catch(error){result.failure=error.message;process.exitCode=1;}
finally{await page.close();await browser.close();await new Promise(done=>server.close(done));}
await writeFile(`${root}/tests/evidence/maintainability/animated-show-result.json`,JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify(result));
