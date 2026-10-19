// Check the complete copied demo and prove that the existing main Web assets
// still match their own receipt. No server or business operations are invoked.
import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
const demo=resolve(fileURLToPath(new URL('../',import.meta.url))),root=resolve(demo,'../..');
const hash=b=>createHash('sha256').update(b).digest('hex');
const read=p=>readFile(resolve(root,p));
const manifest=JSON.parse(await readFile(resolve(demo,'dist/build-manifest.json'),'utf8'));
for(const [path,expected]of Object.entries(manifest.inputs))assert.equal(hash(await read(path)),expected,`source ${path}`);
for(const [path,expected]of Object.entries(manifest.files)){
 assert.equal(hash(await readFile(resolve(demo,'dist',path))),expected,`artifact ${path}`);
 assert.equal(hash(await read(`web/dist/demo/${path}`)),expected,`published ${path}`);
}
assert.deepEqual(await read('web/dist/demo/build-manifest.json'),await readFile(resolve(demo,'dist/build-manifest.json')));
const main=JSON.parse(await read('web/dist/build-manifest.json'));
for(const [path,expected]of Object.entries(main.files))assert.equal(hash(await read(`web/dist/${path}`)),expected,`main Web unchanged: ${path}`);
const browser=JSON.parse(await readFile(resolve(demo,'tests/evidence/browser-result.json')));
const reference=JSON.parse(await readFile(resolve(demo,'tests/evidence/reference/diagrams-result.json')));
const server=JSON.parse(await readFile(resolve(demo,'tests/evidence/server-result.json')));
assert.equal(browser.passed,true);assert.equal(reference.passed,true);assert.equal(server.passed,true);
assert.equal(browser.served_wasm_sha256,manifest.files['pkg/xcs_bg.wasm']);
const result={passed:true,source_inputs:Object.keys(manifest.inputs).length,artifacts:Object.keys(manifest.files).length,existing_main_assets_unchanged:Object.keys(main.files).length,wasm_sha256:manifest.files['pkg/xcs_bg.wasm'],guided_processes:browser.scenarios.length,free_operation_checks:browser.free_operations.length,viewports:browser.viewports.length,static_diagrams:reference.diagrams.length,read_only_preview:server.passed,business_requests:browser.business_requests};
await writeFile(resolve(demo,'tests/evidence/delivery.json'),JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify(result,null,2));
