import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';

const { chromium } = await import(process.env.ZXS_PLAYWRIGHT_MODULE || 'playwright');
const root = resolve(process.env.ZXS_PRODUCT_ROOT || fileURLToPath(new URL('../../../', import.meta.url)));
const source = await readFile(`${root}/tests/evidence/maintainability/browser-probe.mjs`, 'utf8');
// Exercise the same assertion as the real candidate, rather than a weaker copy.
const helper = source.match(/  async function fileRows\(name\) \{[\s\S]*?\n  \}(?=\n  async function loginKnownUser)/)?.[0];
assert.ok(helper, 'real browser fileRows helper must be present');
const makeCheck = new Function('page', 'state', 'geometry', 'assert', `${helper}\nreturn fileRows;`);
const styles = (await Promise.all(
  ['geometry', 'base', 'content-blocks', 'file-manager'].map(name =>
    readFile(`${root}/web/public/styles/components/${name}.css`, 'utf8')
  )
)).join('\n');
const browser = await chromium.launch({ headless: true, args: ['--no-sandbox'] });
const page = await browser.newPage({ viewport: { width: 1440, height: 800 } });
const results = [];

try {
  for (const compact of [36, 40]) {
    for (const kind of ['disk', 'files']) {
      const geometry = { compact, comfortable: compact === 36 ? 44 : 50, ratio: .4 };
      const dom = `<section class="zxs-app-shell">
        <div class="zxs-file-list-panel zxs-frame"><div class="zxs-table-scroll">
          <table class="zxs-file-table">
            <thead><tr><th>名称</th><th>${kind === 'disk' ? '访问权限' : '类型'}</th><th>大小</th><th>操作</th></tr></thead>
            <tbody><tr>
              <td><button class="zxs-button zxs-file-name"><svg viewBox="0 0 24 24"></svg><span>${kind === 'disk' ? 'P01' : '说明.md'}</span></button></td>
              <td>${kind === 'disk' ? '可读写' : '文件'}</td><td>128 B</td>
              <td><div class="zxs-entry-actions zxs-flow"><button class="zxs-button">详情</button></div></td>
            </tr></tbody>
          </table>
        </div></div>
      </section>`;
      for (const fixed of [false, true]) {
        const state = {};
        // Removing only the inherited alignment recreates the observed UA regression.
        const css = styles.replace(/text-align:\s*inherit\s*;/g, '');
        await page.setContent(`<style>${css}
          :root { --zxs-compact-row-height:${compact}px; --zxs-input-row-height:${geometry.comfortable}px; }
          ${fixed ? '.zxs-file-table :is(th,td) { text-align:inherit; }' : ''}
        </style>${dom}`);
        let failure;
        try {
          await makeCheck(page, state, geometry, assert)(`${kind}-${compact}`);
        } catch (error) {
          failure = error.message;
        }
        if (fixed) assert.equal(failure, undefined);
        else assert.match(failure, /must align to the start/);
        results.push({
          kind,
          compact,
          fixed,
          passed: true,
          ...(failure ? { expectedRejection: failure } : { cells: state.fileRows[0].cells })
        });
      }
    }
  }
} finally {
  await browser.close();
}

await writeFile(`${root}/tests/evidence/maintainability/source-table-alignment-result.json`, JSON.stringify({
  passed: true,
  source: 'actual fileRows helper extracted unchanged; static local DOM only, no service/auth',
  checks: results.length,
  results
}, null, 2) + '\n');
console.log(JSON.stringify({ passed: true, negativeControls: 4, positiveControls: 4, noServiceOrAuth: true }));
