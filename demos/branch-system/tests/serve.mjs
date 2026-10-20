// Smoke-test the standalone, read-only Rust preview without a storage pool.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {request} from 'node:http';
import {once} from 'node:events';
import {readFile,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
const demo=resolve(fileURLToPath(new URL('../',import.meta.url))),root=resolve(demo,'../..');
const port=17401;
const child=spawn(resolve(root,'target/web-native/debug/zxs-demo-serve'),['--port',String(port)],{stdio:['ignore','pipe','pipe']});
const get=(path,method='GET')=>new Promise((yes,no)=>{
 const req=request({host:'127.0.0.1',port,path,method},res=>{const chunks=[];res.on('data',c=>chunks.push(c));res.on('end',()=>yes({status:res.statusCode,headers:res.headers,body:Buffer.concat(chunks)}));});req.on('error',no);req.setTimeout(3000,()=>req.destroy(new Error('timeout')));req.end();
});
let stderr='';child.stderr.on('data',data=>stderr+=data);let failure;
try {
 await Promise.race([once(child.stdout,'data'),once(child,'exit').then(()=>{throw Error(`preview failed: ${stderr}`);}),new Promise((_,no)=>setTimeout(()=>no(Error('startup timeout')),5000))]);
 const index=await get('/');assert.equal(index.status,200);assert.deepEqual(index.body,await readFile(resolve(demo,'dist/index.html')));
 const head=await get('/','HEAD');assert.equal(head.status,200);assert.equal(head.body.length,0);assert.equal(Number(head.headers['content-length']),index.body.length);
 assert.equal((await get('/playground.html')).status,200);assert.match((await get('/pkg/zxs_bg.wasm')).headers['content-type'],/^application\/wasm/);
 assert.equal((await get('/../Cargo.toml')).status,404);assert.equal((await get('/%2e%2e/Cargo.toml')).status,404);assert.equal((await get('/','POST')).status,405);
 assert.ok(index.headers['content-security-policy'].includes("'wasm-unsafe-eval'"));assert.ok(index.headers['content-security-policy'].includes('blob:'));
 await writeFile(resolve(demo,'tests/evidence/server-result.json'),JSON.stringify({passed:true,scope:'Standalone Rust preview, no storage root',checks:['guided','playground','HEAD','WASM MIME','strict CSP','traversal rejection','read-only methods']},null,2)+'\n');
 console.log('PASS: standalone read-only Rust demo server');
} catch(e){failure=e;} finally {if(child.exitCode===null && child.signalCode===null){const ended=once(child,'exit');child.kill('SIGTERM');await ended;}}
if(failure)throw failure;
