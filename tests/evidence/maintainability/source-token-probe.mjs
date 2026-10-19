import {fileURLToPath} from 'node:url';
import {resolve} from 'node:path';
const {chromium}=await import(process.env.XCS_PLAYWRIGHT_MODULE||'playwright');
import { readFile, writeFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
const root=resolve(process.env.XCS_PRODUCT_ROOT||fileURLToPath(new URL('../../../',import.meta.url)));
const browser = await chromium.launch({headless: true, args: ['--no-sandbox']});
const results = [];
const html = `<!doctype html><html data-xcs-appearance="content-blocks"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><link rel="stylesheet" href="/styles.css"></head><body>
<div class="xcs-app-shell" data-xcs-scope><main class="xcs-shell-main xcs-stack">
<header class="xcs-page-header xcs-flow"><nav class="xcs-header-navigation xcs-flow"><button class="xcs-button">文件</button></nav><div class="xcs-header-actions xcs-flow xcs-control-group--comfortable"><button class="xcs-button">↻</button></div></header>
<section class="xcs-file-manager xcs-stack"><p class="xcs-row">选择一个硬盘目录</p><div class="xcs-file-toolbar xcs-flow"><button class="xcs-button">上传</button><input class="xcs-input xcs-input--compact" value="搜索"></div><section class="xcs-content-panel xcs-file-list-panel xcs-frame"><table class="xcs-file-table"><tbody><tr><td>文件夹</td><td>目录</td><td>—</td></tr></tbody></table></section></section>
<article class="xcs-entry-content xcs-frame xcs-stack"><h2 class="xcs-row">文件详情</h2><section class="xcs-entry-section xcs-stack xcs-stack--flush"><h3 class="xcs-row">备注</h3><input class="xcs-inline-input xcs-input xcs-input--compact xcs-input--underline" value="备注"></section><article class="xcs-version-node xcs-frame xcs-stack xcs-stack--flush"><time class="xcs-row">2026-10-19</time><p class="xcs-row">版本备注</p><div class="xcs-version-actions xcs-flow"><button class="xcs-button">编辑备注</button><button class="xcs-button">创建分支</button></div></article></article>
<div class="xcs-history-list-panel xcs-frame"><ul class="xcs-activity-list"><li class="xcs-history-row xcs-flow xcs-flow--stretch"><span class="xcs-history-action xcs-row">保存</span><span class="xcs-history-path xcs-row">P01/文件.md</span><time class="xcs-row">2026-10-19</time></li></ul></div>
<div class="xcs-metric-grid"><article class="xcs-metric-card xcs-frame xcs-frame--inset"><header class="xcs-metric-heading xcs-row xcs-row--flush">CPU</header><p class="xcs-metric-subtitle xcs-row xcs-row--flush">1.0%</p><div class="xcs-metric-chart"></div></article></div>
<section class="xcs-account-page xcs-stack"><div class="xcs-content-panel xcs-frame xcs-stack"><form class="xcs-account-form xcs-stack"><div class="xcs-account-fields"><label class="xcs-form-field"><span class="xcs-row xcs-row--center">用户名</span><input class="xcs-input xcs-input--compact" readonly value="admin"></label></div><div class="xcs-actions xcs-flow"><button class="xcs-button">保存</button></div></form></div></section>
<label><input class="xcs-input" value="普通输入"></label><div class="xcs-select"><span class="xcs-select-label xcs-row">分支</span><button class="xcs-input xcs-input--compact xcs-select-trigger">main</button><div class="xcs-select-panel xcs-frame"><div class="xcs-select-option xcs-row xcs-row--option">main</div></div></div>
</main></div>
<div class="xcs-auth-shell"><div class="xcs-auth-card"><form><h1>登录</h1><label class="xcs-form-field"><span>用户名</span><input name="username" class="xcs-input"></label><label class="xcs-form-field"><span>密码</span><input name="password" class="xcs-input" type="password"></label><button class="xcs-button">登录</button></form></div></div>
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
    const box = e => { const r=e.getBoundingClientRect(),s=getComputedStyle(e); return {height:r.height,font:parseFloat(s.fontSize),left:parseFloat(s.paddingLeft),extent:parseFloat(s.getPropertyValue('--xcs-contour-extent')),gap:parseFloat(s.rowGap),border:parseFloat(s.borderLeftWidth)}; };
    return {rows:[...document.querySelectorAll('.xcs-app-shell .xcs-row')].filter(e=>!e.classList.contains('xcs-row--flush')).map(box),buttons:[...document.querySelectorAll('.xcs-app-shell .xcs-button')].map(box),inputs:[...document.querySelectorAll('.xcs-app-shell .xcs-input')].map(e=>({...box(e),compact:e.classList.contains('xcs-input--compact')})),frames:[...document.querySelectorAll('.xcs-frame')].map(box),stacks:[...document.querySelectorAll('.xcs-app-shell .xcs-stack:not(.xcs-stack--flush)')].map(box),flows:[...document.querySelectorAll('.xcs-flow')].map(box),cell:box(document.querySelector('.xcs-file-table td')),auth:[...document.querySelectorAll('.xcs-auth-card,.xcs-auth-card form,.xcs-auth-card input,.xcs-auth-card button')].map(box),overflow:document.documentElement.scrollWidth>innerWidth};
  });
  const before = await snapshot();
  assert.equal(before.overflow, false);
  await page.addStyleTag({content: ':root { --xcs-compact-row-height:40px; --xcs-input-row-height:50px; }'});
  const after = await snapshot();
  for (const row of after.rows) { assert.ok(row.height>=39.9); assert.ok(Math.abs(row.font-16)<.02); }
  for (const frame of after.frames) { assert.equal(frame.extent,20); assert.equal(frame.border,1); }
  for (const stack of after.stacks) assert.equal(stack.gap,20);
  for (const flow of after.flows) assert.equal(flow.gap,20);
  for (const input of after.inputs) { assert.equal(input.height,input.compact?40:50); assert.ok(Math.abs(input.font-(input.compact?16:20))<.02); if (input.extent) assert.equal(input.extent,input.compact?20:25); }
  assert.equal(after.cell.height,40);
  const option=await page.locator('.xcs-select-option').evaluate(e=>({height:e.getBoundingClientRect().height,padding:parseFloat(getComputedStyle(e).paddingLeft),font:parseFloat(getComputedStyle(e).fontSize)}));
  assert.equal(option.height,40); assert.equal(option.font,16); assert.equal(option.padding,11);
  const underline=await page.locator('.xcs-input--underline').evaluate(e=>parseFloat(getComputedStyle(e).paddingLeft)); assert.equal(underline,11);
  assert.equal(after.buttons.at(0).font,16);
  assert.equal(after.buttons.at(1).height,50);
  assert.equal(after.buttons.at(1).font,20);
  assert.deepEqual(after.auth,before.auth,'business row tokens must not resize authentication');
  assert.equal(after.overflow,false);
  results.push({width,before,after});
  await page.close();
}
await browser.close();
await writeFile('/tmp/xcs-geometry-token-result.json',JSON.stringify({passed:true,results},null,2)+'\n');
console.log('PASS shared geometry: live token change 36→40 and 44→50 propagates rows/fonts/G4/frames/inputs/header/gaps across all pages, while login remains identical, at 4 normal widths.');
