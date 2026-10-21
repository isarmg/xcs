// Historical evidence probe, not current acceptance. See tests/evidence/README.md.
import {configureProbe, loadPlaywright, fixtureOrigin, requiredInput} from '../_support/runtime.mjs';
const probe = await configureProbe(import.meta.url);
import {fileURLToPath} from 'node:url';
import {resolve} from 'node:path';
const {chromium}=await loadPlaywright();
import { readFile, writeFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
const root=probe.root;
const browser = await chromium.launch({headless: true, args: ['--no-sandbox']});
const results = [];
const html = `<!doctype html><html data-zxs-appearance="content-blocks"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><link rel="stylesheet" href="/styles.css"></head><body>
<div class="zxs-app-shell" data-zxs-scope><main class="zxs-shell-main zxs-stack">
<header class="zxs-page-header zxs-flow"><nav class="zxs-header-navigation zxs-flow"><button class="zxs-button">文件</button></nav><div class="zxs-header-actions zxs-flow zxs-control-group--comfortable"><button class="zxs-button">↻</button></div></header>
<section class="zxs-file-manager zxs-stack"><p class="zxs-row">选择一个硬盘目录</p><div class="zxs-file-toolbar zxs-flow"><button class="zxs-button">上传</button><input class="zxs-input zxs-input--compact" value="搜索"></div><section class="zxs-content-panel zxs-file-list-panel zxs-frame"><table class="zxs-file-table"><tbody><tr><td>文件夹</td><td>目录</td><td>—</td></tr></tbody></table></section></section>
<article class="zxs-entry-content zxs-frame zxs-stack"><h2 class="zxs-row">文件详情</h2><section class="zxs-entry-section zxs-stack zxs-stack--flush"><h3 class="zxs-row">备注</h3><input class="zxs-inline-input zxs-input zxs-input--compact zxs-input--underline" value="备注"></section><article class="zxs-version-node zxs-frame zxs-stack zxs-stack--flush"><time class="zxs-row">2026-10-19</time><p class="zxs-row">版本备注</p><div class="zxs-version-actions zxs-flow"><button class="zxs-button">编辑备注</button><button class="zxs-button">创建分支</button></div></article></article>
<div class="zxs-history-list-panel zxs-frame"><ul class="zxs-activity-list"><li class="zxs-history-row zxs-flow zxs-flow--stretch"><span class="zxs-history-action zxs-row">保存</span><span class="zxs-history-path zxs-row">P01/文件.md</span><time class="zxs-row">2026-10-19</time></li></ul></div>
<div class="zxs-metric-grid"><article class="zxs-metric-card zxs-frame zxs-frame--inset"><header class="zxs-metric-heading zxs-row zxs-row--flush">CPU</header><p class="zxs-metric-subtitle zxs-row zxs-row--flush">1.0%</p><div class="zxs-metric-chart"></div></article></div>
<section class="zxs-account-page zxs-stack"><div class="zxs-content-panel zxs-frame zxs-stack"><form class="zxs-account-form zxs-stack"><div class="zxs-account-fields"><label class="zxs-form-field"><span class="zxs-row zxs-row--center">用户名</span><input class="zxs-input zxs-input--compact" readonly value="admin"></label></div><div class="zxs-actions zxs-flow"><button class="zxs-button">保存</button></div></form></div></section>
<label><input class="zxs-input" value="普通输入"></label><div class="zxs-select"><span class="zxs-select-label zxs-row">分支</span><button class="zxs-input zxs-input--compact zxs-select-trigger">main</button><div class="zxs-select-panel zxs-frame"><div class="zxs-select-option zxs-row zxs-row--option">main</div></div></div>
</main></div>
<div class="zxs-auth-shell"><div class="zxs-auth-card"><form><h1>登录</h1><label class="zxs-form-field"><span>用户名</span><input name="username" class="zxs-input"></label><label class="zxs-form-field"><span>密码</span><input name="password" class="zxs-input" type="password"></label><button class="zxs-button">登录</button></form></div></div>
</body></html>`;
for (const width of [1440,800,390,320]) {
  const page = await browser.newPage({viewport: {width, height: 1200}});
  await page.route('http://geometry.fixture/**', async route => {
    const path = new URL(route.request().url()).pathname;
    if (path === '/') return route.fulfill({contentType: 'text/html', body: html});
    try { return route.fulfill({contentType: path.endsWith('.css') ? 'text/css' : 'font/woff2', body: await readFile(path === '/styles.css' ? `${root}/web/styles.css` : `${root}/web/public${path}`)}); }
    catch { return route.fulfill({status: 404, body: ''}); }
  });
  await page.goto('http://geometry.fixture/');
  await page.evaluate(() => document.fonts.ready);
  const snapshot = () => page.evaluate(() => {
    const box = e => { const r=e.getBoundingClientRect(),s=getComputedStyle(e); return {height:r.height,font:parseFloat(s.fontSize),left:parseFloat(s.paddingLeft),extent:parseFloat(s.getPropertyValue('--zxs-contour-extent')),gap:parseFloat(s.rowGap),border:parseFloat(s.borderLeftWidth)}; };
    return {rows:[...document.querySelectorAll('.zxs-app-shell .zxs-row')].filter(e=>!e.classList.contains('zxs-row--flush')).map(box),buttons:[...document.querySelectorAll('.zxs-app-shell .zxs-button')].map(box),inputs:[...document.querySelectorAll('.zxs-app-shell .zxs-input')].map(e=>({...box(e),compact:e.classList.contains('zxs-input--compact')})),frames:[...document.querySelectorAll('.zxs-frame')].map(box),stacks:[...document.querySelectorAll('.zxs-app-shell .zxs-stack:not(.zxs-stack--flush)')].map(box),flows:[...document.querySelectorAll('.zxs-flow')].map(box),cell:box(document.querySelector('.zxs-file-table td')),auth:[...document.querySelectorAll('.zxs-auth-card,.zxs-auth-card form,.zxs-auth-card input,.zxs-auth-card button')].map(box),overflow:document.documentElement.scrollWidth>innerWidth};
  });
  const before = await snapshot();
  assert.equal(before.overflow, false);
  await page.addStyleTag({content: ':root { --zxs-compact-row-height:40px; --zxs-input-row-height:50px; }'});
  const after = await snapshot();
  for (const row of after.rows) { assert.ok(row.height>=39.9); assert.ok(Math.abs(row.font-16)<.02); }
  for (const frame of after.frames) { assert.equal(frame.extent,20); assert.equal(frame.border,1); }
  for (const stack of after.stacks) assert.equal(stack.gap,20);
  for (const flow of after.flows) assert.equal(flow.gap,20);
  for (const input of after.inputs) { assert.equal(input.height,input.compact?40:50); assert.ok(Math.abs(input.font-(input.compact?16:20))<.02); if (input.extent) assert.equal(input.extent,input.compact?20:25); }
  assert.equal(after.cell.height,40);
  const option=await page.locator('.zxs-select-option').evaluate(e=>({height:e.getBoundingClientRect().height,padding:parseFloat(getComputedStyle(e).paddingLeft),font:parseFloat(getComputedStyle(e).fontSize)}));
  assert.equal(option.height,40); assert.equal(option.font,16); assert.equal(option.padding,11);
  const underline=await page.locator('.zxs-input--underline').evaluate(e=>parseFloat(getComputedStyle(e).paddingLeft)); assert.equal(underline,11);
  assert.equal(after.buttons.at(0).font,16);
  assert.equal(after.buttons.at(1).height,50);
  assert.equal(after.buttons.at(1).font,20);
  assert.deepEqual(after.auth,before.auth,'business row tokens must not resize authentication');
  assert.equal(after.overflow,false);
  results.push({width,before,after});
  await page.close();
}
await browser.close();
await writeFile(`${probe.output}/zxs-geometry-token-result.json`,JSON.stringify({passed:true,results},null,2)+'\n');
console.log('PASS shared geometry: live token change 36→40 and 44→50 propagates rows/fonts/G4/frames/inputs/header/gaps across all pages, while login remains identical, at 4 normal widths.');
