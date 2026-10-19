import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdir, readFile, writeFile} from 'node:fs/promises';
import {dirname, resolve} from 'node:path';
import {fileURLToPath} from 'node:url';

const {chromium} = await import(process.env.XCS_PLAYWRIGHT_MODULE || 'playwright');
const evidence = dirname(fileURLToPath(import.meta.url));
const root = resolve(process.env.XCS_PRODUCT_ROOT || resolve(evidence, '../../..'));
const authPath = 'web/public/styles/components/auth.css';
// Captured before this correction. Business spacing must leave authentication alone.
const authBefore = 'c5d671093294a9cf6b8384aae66c819b0f43afe2400d70b09467fe10e2178f96';
const hashes = new Map();
const results = [];
let negativeControl;
const browser = await chromium.launch({headless: true, args: ['--no-sandbox']});
const hash = body => createHash('sha256').update(body).digest('hex');
const near = (actual, expected, label, tolerance = .2) =>
  assert.ok(Math.abs(actual - expected) <= tolerance, `${label}: ${actual}; expected ${expected}`);

function select(id, label) {
  return `<div id="${id}" class="xcs-select">
    <label class="xcs-select-label xcs-row">${label}</label>
    <button class="xcs-input xcs-select-trigger xcs-input--compact"><span class="xcs-select-value">main</span><span>›</span></button>
  </div>`;
}

function table() {
  return `<section class="xcs-content-panel xcs-file-list-panel xcs-frame xcs-stack xcs-stack--flush">
    <div class="xcs-table-scroll"><table class="xcs-table xcs-file-table"><thead><tr><th>名称</th><th>类型</th><th>大小</th><th>操作</th></tr></thead>
      <tbody><tr><td><button class="xcs-button xcs-file-name"><span>说明.md</span></button></td><td>文件</td><td>472 B</td><td><div class="xcs-entry-actions xcs-flow"><button class="xcs-button">详情</button></div></td></tr></tbody>
    </table></div>
  </section>`;
}

function fileSection(lang, readonly = false) {
  const en = lang === 'en';
  const id = readonly ? 'readonly' : 'files';
  return `<section id="${id}" class="xcs-content-stack xcs-file-manager xcs-stack">
    <nav class="xcs-breadcrumb xcs-flow"><button class="xcs-button">${en ? 'All disks' : '所有硬盘'}</button><span>/</span><button class="xcs-button">P01</button></nav>
    <div class="xcs-file-toolbar xcs-flow" data-toolbar="actions">
      <button class="xcs-button">${en ? 'Upload files' : '上传文件'}</button>
      <span class="xcs-inline-edit xcs-flow xcs-flow--inline"><button class="xcs-button">${en ? 'New folder' : '新建文件夹'}</button></span>
      <span class="xcs-inline-edit xcs-flow xcs-flow--inline"><button class="xcs-button" disabled>${en ? 'Rename' : '重命名'}</button></span>
      <button class="xcs-button" disabled>${en ? 'Delete' : '删除'}</button>
      <span class="xcs-inline-branch xcs-flow xcs-flow--inline"><button class="xcs-button">${en ? 'Create folder branch' : '创建目录分支'}</button></span>
      <label class="xcs-file-search"><span class="xcs-visually-hidden">${en ? 'Search this folder' : '搜索当前文件夹'}</span><input class="xcs-input xcs-input--compact" type="search" placeholder="${en ? 'Search this folder' : '搜索当前文件夹'}"></label>
    </div>
    <div class="xcs-file-toolbar xcs-flow xcs-surface-end" data-toolbar="branch">
      ${select(`${id}-select`, en ? 'Current folder branch' : '当前目录分支')}
      <p class="xcs-row">${en ? 'Switches the matching branch of each file in this folder and its subfolders.' : '只批量切换此目录及子目录内文件的同名分支。'}</p>
    </div>
    ${readonly ? `<p class="xcs-row" data-readonly>${en ? 'This disk directory is read only.' : '此硬盘目录当前只读。'}</p>` : ''}
    <div class="xcs-file-layout xcs-stack xcs-surface">${table()}</div>
  </section>`;
}

function statusSection(lang) {
  const en = lang === 'en';
  const cards = Array.from({length: 9}, (_, i) => `<article class="xcs-content-panel xcs-metric-card xcs-frame xcs-frame--inset">
    <header class="xcs-metric-heading xcs-row xcs-row--flush"><span class="xcs-metric-title"><span>${i ? 'P01' : en ? 'CPU usage' : 'CPU 使用率'}</span></span><strong class="xcs-metric-current">5.2%</strong></header>
    <p class="xcs-metric-subtitle xcs-row xcs-row--flush"><span>68.8 GiB / 78.4 GiB</span></p><div class="xcs-metric-chart"></div>
  </article>`).join('');
  return `<section id="status" class="xcs-content-stack xcs-monitor-page xcs-stack">
    <div class="xcs-monitor-toolbar xcs-flow xcs-surface-end"><p class="xcs-row">${en ? 'CPU, memory and disk status refresh every 5 seconds.' : '每 5 秒刷新 CPU、内存和硬盘状态。'}</p><div class="xcs-monitor-range">${select('range-select', en ? 'History range' : '历史范围')}</div></div>
    <div class="xcs-metric-grid xcs-surface-start">${cards}</div>
    <p class="xcs-metric-timestamp xcs-row">${en ? 'Sampled at' : '采样时间'}: 2026-10-19</p>
  </section>`;
}

function historySection(lang) {
  const en = lang === 'en';
  return `<section id="history" class="xcs-content-stack xcs-history-page xcs-stack">
    <p class="xcs-row">${en ? 'The latest committed file operations are shown.' : '显示最近已提交的文件操作。'}</p>
    <div class="xcs-history-toolbar xcs-flow xcs-flow--end xcs-surface-end">${select('history-select', en ? 'Disk' : '硬盘')}</div>
    <div class="xcs-content-panel xcs-history-list-panel xcs-frame xcs-stack xcs-stack--flush"><ul class="xcs-activity-list xcs-history-list">
      <li class="xcs-history-row xcs-flow xcs-flow--stretch"><span class="xcs-history-action xcs-row">${en ? 'Created folder branches' : '创建目录分支'}</span><span class="xcs-history-path xcs-row">P01/示例文件_20261018/说明.md</span><time class="xcs-row">2026-10-19 12:39:42</time><span class="xcs-history-principal xcs-row">admin</span></li>
      <li class="xcs-history-row xcs-flow xcs-flow--stretch"><span class="xcs-history-action xcs-row">${en ? 'Saved file' : '保存文件'}</span><span class="xcs-history-path xcs-row">P01/说明.md</span></li>
    </ul></div>
  </section>`;
}

function detailSection(lang) {
  const en = lang === 'en';
  const node = (id, date) => `<article id="${id}" class="xcs-content-panel xcs-version-node xcs-frame xcs-stack xcs-stack--flush"><time class="xcs-row">${date}</time><p class="xcs-row">${en ? 'Version note' : '版本备注'}</p><div class="xcs-version-actions xcs-flow"><button class="xcs-button">${en ? 'Edit note' : '编辑备注'}</button><button class="xcs-button">${en ? 'Create branch' : '创建分支'}</button></div></article>`;
  return `<section id="detail" class="xcs-content-stack xcs-entry-page xcs-stack">
    <nav class="xcs-entry-navigation xcs-flow"><button class="xcs-button">${en ? 'Back to files' : '返回文件'}</button><span>P01/说明.md</span></nav>
    <article class="xcs-content-panel xcs-entry-content xcs-frame xcs-stack">
      <div class="xcs-file-heading xcs-flow"><h2 class="xcs-row">说明.md</h2><div class="xcs-entry-actions xcs-flow"><button class="xcs-button">${en ? 'Rename' : '重命名'}</button></div></div>
      <dl class="xcs-file-properties"><div><dt class="xcs-row">${en ? 'Location' : '位置'}</dt><dd class="xcs-row">P01/说明.md</dd></div><div><dt class="xcs-row">${en ? 'Type' : '类型'}</dt><dd class="xcs-row">${en ? 'File' : '文件'}</dd></div></dl>
      <section class="xcs-entry-section xcs-stack xcs-stack--flush"><h3 class="xcs-row">${en ? 'File note' : '文件备注'}</h3><span class="xcs-inline-edit xcs-flow xcs-flow--inline"><button class="xcs-button">${en ? 'Edit file note' : '编辑文件备注'}</button></span></section>
      <section class="xcs-entry-section xcs-entry-version-history xcs-stack"><h3 class="xcs-row">${en ? 'Version history' : '版本历史'}</h3><div class="xcs-version-tree-scroll"><div class="xcs-version-tree-canvas"><div class="xcs-version-tree-level">${node('newer', '2026-10-19')}${node('sibling', '2026-10-18')}</div><div class="xcs-version-tree-level">${node('older', '2026-10-17')}</div></div></div></section>
    </article>
  </section>`;
}

function accountSection(lang) {
  const en = lang === 'en';
  const fields = [['username', en ? 'Username' : '用户名', 'text'], ['current_password', en ? 'Current password' : '当前密码', 'password'], ['new_password', en ? 'New password' : '新密码', 'password'], ['confirm_password', en ? 'Confirm new password' : '确认新密码', 'password']]
    .map(([name, label, type]) => `<label class="xcs-form-field"><span class="xcs-row xcs-row--center">${label}</span><input class="xcs-input xcs-input--compact" name="${name}" type="${type}" ${name === 'username' ? 'value="fixture-admin" readonly' : ''}></label>`).join('');
  return `<section id="account" class="xcs-content-stack xcs-account-page xcs-stack"><p class="xcs-row">${en ? 'Changing the password signs out all sessions.' : '修改密码后，全部会话将失效。'}</p><div class="xcs-content-panel xcs-frame xcs-stack"><form class="xcs-account-form xcs-stack"><div class="xcs-account-fields">${fields}</div><div class="xcs-actions xcs-flow"><button class="xcs-button">${en ? 'Save' : '保存'}</button></div></form></div></section>`;
}

function boundarySection() {
  const frame = content => `<div class="xcs-frame"><p class="xcs-row">${content}</p></div>`;
  return `<section id="boundaries">
    <div id="touching" class="xcs-stack">${frame('A')}${frame('B')}</div>
    <div id="separated" class="xcs-stack">${frame('A')}<p class="xcs-row">Natural text row</p>${frame('B')}</div>
    <div id="bare" class="xcs-stack"><p class="xcs-row">First text row</p><p class="xcs-row">Second text row</p></div>
    <div id="wrapped" class="xcs-flow"><button class="xcs-button">First text item</button><button class="xcs-button">Second text item</button><button class="xcs-button">Third text item</button><button class="xcs-button">Fourth text item</button><button class="xcs-button">Fifth text item</button></div>
  </section>`;
}

function authSection(lang) {
  const en = lang === 'en';
  return `<div id="auth" class="xcs-auth-shell"><div class="xcs-auth-card"><form><h1>${en ? 'Sign in' : '登录'}</h1><label class="xcs-form-field"><span>${en ? 'Username' : '用户名'}</span><input name="username" class="xcs-input"></label><label class="xcs-form-field"><span>${en ? 'Password' : '密码'}</span><input name="password" class="xcs-input" type="password"></label><button class="xcs-button">${en ? 'Sign in' : '登录'}</button></form></div></div>`;
}

function markup(lang, authOnly = false) {
  const body = authOnly ? '' : `<div class="xcs-app-shell" data-xcs-scope><main class="xcs-shell-main">
    ${fileSection(lang)}${fileSection(lang, true)}${historySection(lang)}${statusSection(lang)}${detailSection(lang)}${accountSection(lang)}${boundarySection()}
  </main></div>`;
  return `<!doctype html><html lang="${lang}" data-xcs-appearance="content-blocks" data-xcs-theme="light"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><link rel="stylesheet" href="/styles.css"></head><body>${body}${authSection(lang)}</body></html>`;
}

async function installFixture(page, lang, {authOnly = false, legacySpacing = false} = {}) {
  await page.route('http://spacing.fixture/**', async route => {
    const path = new URL(route.request().url()).pathname;
    if (path === '/') return route.fulfill({contentType: 'text/html', body: markup(lang, authOnly)});
    const file = path === '/styles.css' ? 'web/styles.css' : `web/public${path}`;
    try {
      let body = await readFile(resolve(root, file));
      hashes.set(file, hash(body));
      if (legacySpacing && path.endsWith('/geometry.css')) {
        // Reproduce the former shared gap settings, retaining @property and all
        // unrelated geometry so this control isolates the spacing correction.
        body = Buffer.from(body.toString()
          .replace(/(\.xcs-app-shell \.xcs-stack\s*\{[^}]*?)gap:\s*0;/, '$1gap: var(--xcs-block-gap);')
          .replace(/(\.xcs-app-shell \.xcs-flow\s*\{[^}]*?)row-gap:\s*0;/, '$1row-gap: var(--xcs-block-gap);'));
      }
      return route.fulfill({contentType: path.endsWith('.css') ? 'text/css' : 'font/woff2', body});
    } catch {
      return route.fulfill({status: 404, body: ''});
    }
  });
  await page.goto('http://spacing.fixture/');
  await page.evaluate(() => document.fonts.ready);
}

async function authGeometry(page) {
  return page.locator('#auth').evaluate(shell => {
    const origin = shell.getBoundingClientRect();
    return [...shell.querySelectorAll('.xcs-auth-card,.xcs-auth-card form,.xcs-form-field>span,input,button')].map(element => {
      const b = element.getBoundingClientRect();
      const s = getComputedStyle(element);
      return {tag: element.tagName, x: b.x - origin.x, y: b.y - origin.y, width: b.width, height: b.height, font: s.fontSize, lineHeight: s.lineHeight, padding: [s.paddingTop, s.paddingRight, s.paddingBottom, s.paddingLeft], contour: s.getPropertyValue('--xcs-contour-extent').trim()};
    });
  });
}

function checkFile(value, label, width) {
  near(value.actions.top - value.breadcrumb.bottom, 0, `${label}: breadcrumb/actions extra gap`);
  near(value.branch.top - value.actions.bottom, 0, `${label}: actions/branch extra gap`);
  near(value.label.top - value.branch.top, 0, `${label}: label begins branch row`);
  near(value.select.top - value.label.bottom, 0, `${label}: select/label extra gap`);
  assert.ok(value.select.top - value.search.bottom >= 35.8, `${label}: select already separated from search by its label row`);
  near(value.stackGap, 0, `${label}: stack default gap`);
  near(value.actionsGap, 0, `${label}: action wrapping gap`);
  near(value.branchGap, 0, `${label}: branch wrapping gap`);
  if (value.readonly) {
    near(value.readonly.top - value.branch.bottom, 0, `${label}: readonly row starts immediately`);
    near(value.frame.top - value.readonly.bottom, 0, `${label}: natural readonly row prevents extra gap`);
    assert.ok(value.frame.top - value.select.bottom >= 35.8, `${label}: natural row gives physical separation`);
  } else if (width > 504) {
    near(value.frame.top - value.branch.bottom, 18, `${label}: touching branch/list surface clearance`);
    near(value.frame.top - value.select.bottom, 18, `${label}: physical select/list surface clearance`);
  } else {
    assert.ok(value.hint.top >= value.select.bottom - .2, `${label}: phone hint starts after select`);
    assert.ok(value.hint.height >= 35.8, `${label}: hint provides natural separation`);
    near(value.frame.top - value.hint.bottom, 0, `${label}: no gap after phone hint`);
  }
}

try {
  assert.equal(hash(await readFile(resolve(root, authPath))), authBefore, 'authentication CSS changed during spacing correction');
  const oldGapPage = await browser.newPage({viewport: {width: 1440, height: 1100}});
  await installFixture(oldGapPage, 'zh', {legacySpacing: true});
  const oldGap = await oldGapPage.locator('#files').evaluate(element => {
    const breadcrumb = element.querySelector('.xcs-breadcrumb').getBoundingClientRect();
    const toolbar = element.querySelector('[data-toolbar=actions]').getBoundingClientRect();
    return {computedStackGap: parseFloat(getComputedStyle(element).rowGap), actualUnframedGap: toolbar.top - breadcrumb.bottom};
  });
  near(oldGap.computedStackGap, 18, 'negative control recreates former blanket stack gap');
  near(oldGap.actualUnframedGap, 18, 'negative control recreates former unframed-row clearance');
  assert.throws(() => near(oldGap.actualUnframedGap, 0, 'negative control must reject unnecessary gap'), /negative control must reject unnecessary gap/);
  negativeControl = {...oldGap, oldRuleRejectedByNewRequirement: true};
  await oldGapPage.close();
  for (const lang of ['zh', 'en']) for (const width of [1440, 800, 390, 320]) {
    const label = `${lang}/${width}`;
    const page = await browser.newPage({viewport: {width, height: 1100}});
    await installFixture(page, lang);
    const data = await page.evaluate(() => {
      const b = element => {
        const r = element.getBoundingClientRect();
        const s = getComputedStyle(element);
        return {top: r.top, bottom: r.bottom, left: r.left, right: r.right, width: r.width, height: r.height, marginTop: parseFloat(s.marginTop), gap: parseFloat(s.rowGap) || 0, padding: [s.paddingTop, s.paddingRight, s.paddingBottom, s.paddingLeft], font: parseFloat(s.fontSize)};
      };
      const q = selector => document.querySelector(selector);
      const file = id => {
        const element = q(`#${id}`);
        const e = selector => element.querySelector(selector);
        return {breadcrumb: b(e('.xcs-breadcrumb')), actions: b(e('[data-toolbar=actions]')), branch: b(e('[data-toolbar=branch]')), label: b(e('.xcs-select-label')), select: b(e('.xcs-select-trigger')), search: b(e('.xcs-file-search input')), hint: b(e('[data-toolbar=branch] > p')), frame: b(e('.xcs-file-list-panel')), readonly: e('[data-readonly]') ? b(e('[data-readonly]')) : null, stackGap: b(element).gap, actionsGap: b(e('[data-toolbar=actions]')).gap, branchGap: b(e('[data-toolbar=branch]')).gap};
      };
      return {
        overflow: document.documentElement.scrollWidth > innerWidth,
        files: file('files'), readonly: file('readonly'),
        status: {toolbar: b(q('.xcs-monitor-toolbar')), range: b(q('.xcs-monitor-range .xcs-select-trigger')), grid: b(q('.xcs-metric-grid')), cards: [...document.querySelectorAll('.xcs-metric-card')].map(b)},
        history: {toolbar: b(q('.xcs-history-toolbar')), trigger: b(q('#history-select .xcs-select-trigger')), frame: b(q('.xcs-history-list-panel')), rows: [...document.querySelectorAll('.xcs-history-row')].map(element => ({...b(element), children: [...element.children].map(b)}))},
        detail: {frame: b(q('.xcs-entry-content')), heading: b(q('.xcs-file-heading')), properties: b(q('.xcs-file-properties')), sections: [...document.querySelectorAll('.xcs-entry-section')].map(b)},
        tree: {canvas: b(q('.xcs-version-tree-canvas')), levels: [...document.querySelectorAll('.xcs-version-tree-level')].map(b), newer: b(q('#newer')), sibling: b(q('#sibling')), older: b(q('#older'))},
        account: {fields: b(q('.xcs-account-fields')), form: b(q('.xcs-account-form')), labels: [...document.querySelectorAll('#account .xcs-form-field > span')].map(b), inputs: [...document.querySelectorAll('#account input')].map(b), actions: b(q('#account .xcs-actions'))},
        boundaries: Object.fromEntries(['touching', 'separated', 'bare', 'wrapped'].map(id => [id, {container: b(q(`#${id}`)), children: [...q(`#${id}`).children].map(b)}])),
      };
    });
    assert.equal(data.overflow, false, `${label}: document overflow`);
    checkFile(data.files, `${label}/files`, width);
    checkFile(data.readonly, `${label}/readonly`, width);

    near(data.status.grid.top - data.status.toolbar.bottom, 18, `${label}: status toolbar/grid boundary`);
    near(data.status.grid.top - data.status.range.bottom, 18, `${label}: physical range/card clearance`);
    near(data.status.grid.gap, 18, `${label}: status card grid gap`);
    const firstTop = data.status.cards[0].top;
    const firstRow = data.status.cards.filter(card => Math.abs(card.top - firstTop) < .2);
    assert.equal(firstRow.length, width > 768 ? 4 : 1, `${label}: status equal slots`);
    for (const card of data.status.cards) {
      near(card.width / card.height, 1.5, `${label}: card aspect ratio`, .002);
      near(card.width, data.status.cards[0].width, `${label}: final card same width`);
    }
    if (width > 768) near(firstRow[1].left - firstRow[0].right, 18, `${label}: horizontal card clearance`);
    const secondRow = data.status.cards.find(card => card.top > firstTop + 1);
    near(secondRow.top - data.status.cards[0].bottom, 18, `${label}: vertical card clearance`);

    near(data.history.frame.top - data.history.toolbar.bottom, 18, `${label}: history toolbar/list boundary`);
    for (const row of data.history.rows) {
      near(row.gap, 0, `${label}: history text wrapping has no extra gap`);
      const levels = [...new Set(row.children.map(child => child.top))].sort((a, b) => a - b);
      for (let index = 1; index < levels.length; index++) {
        const previousBottom = Math.max(...row.children.filter(child => Math.abs(child.top - levels[index - 1]) < .2).map(child => child.bottom));
        near(levels[index] - previousBottom, 0, `${label}: actual history text wrapping clearance`);
      }
    }
    near(data.detail.frame.gap, 0, `${label}: detail container no blanket gap`);
    near(data.detail.properties.top - data.detail.heading.bottom, 0, `${label}: detail heading/properties natural rows`);
    for (const section of data.detail.sections) near(section.gap, 0, `${label}: detail section no blanket gap`);
    near(data.tree.canvas.gap, 18, `${label}: tree level gap retained`);
    near(data.tree.sibling.left - data.tree.newer.right, 18, `${label}: tree sibling clearance`);
    near(data.tree.levels[1].top - data.tree.levels[0].bottom, 18, `${label}: tree physical level clearance`);

    near(data.account.form.gap, 0, `${label}: account form no blanket gap`);
    near(data.account.actions.top - data.account.fields.bottom, 0, `${label}: account save row no extra gap`);
    near(data.account.fields.gap, width > 768 ? 18 : 0, `${label}: account input separation rule`);
    for (let index = 1; index < data.account.inputs.length; index++) {
      if (width > 768) {
        near(data.account.inputs[index].top - data.account.inputs[index - 1].bottom, 18, `${label}: desktop input clearance`);
      } else {
        near(data.account.labels[index].top - data.account.inputs[index - 1].bottom, 0, `${label}: phone label natural separation`);
        near(data.account.inputs[index].top - data.account.labels[index].bottom, 0, `${label}: phone label/input extra gap`);
        near(data.account.inputs[index].top - data.account.inputs[index - 1].bottom, 36, `${label}: phone inputs separated by one label row`);
      }
    }
    near(data.boundaries.touching.children[1].top - data.boundaries.touching.children[0].bottom, 18, `${label}: adjacent visible frames minimum clearance`);
    for (const id of ['separated', 'bare']) {
      const children = data.boundaries[id].children;
      for (let index = 1; index < children.length; index++) near(children[index].top - children[index - 1].bottom, 0, `${label}: ${id} no duplicated external gap`);
    }
    near(data.boundaries.wrapped.container.gap, 0, `${label}: unframed wrap gap`);

    // Compare the same isolated login DOM before/after shared business gap values.
    const loginWith = await browser.newPage({viewport: {width, height: 1100}});
    const loginWithout = await browser.newPage({viewport: {width, height: 1100}});
    await installFixture(loginWith, lang, {authOnly: true});
    await installFixture(loginWithout, lang, {authOnly: true, legacySpacing: true});
    const login = await authGeometry(loginWith);
    assert.deepEqual(login, await authGeometry(loginWithout), `${label}: shared business spacing must not change login geometry`);
    await loginWith.close();
    await loginWithout.close();

    await page.locator('#files').screenshot({path: resolve(evidence, `files-${lang}-${width}.png`)});
    await page.locator('#status').screenshot({path: resolve(evidence, `status-${lang}-${width}.png`)});
    await page.locator('#account').screenshot({path: resolve(evidence, `account-${lang}-${width}.png`)});
    results.push({lang, width, passed: true, data, login});
    console.log(`PASS ${label}: natural rows, visible-frame clearance, phone labels, cards/tree/history/account, isolated login.`);
    await page.close();
  }
  await mkdir(evidence, {recursive: true});
  await writeFile(resolve(evidence, 'source-spacing-result.json'), JSON.stringify({passed: true, evidenceKind: 'source-css-dom-fixture', authenticatesRealService: false, businessMutations: 0, browser: 'Chromium', scenes: results.length, negativeControl, authCssUnchanged: true, authCssSha256: authBefore, sourceHashes: Object.fromEntries([...hashes].sort(([a], [b]) => a.localeCompare(b))), results}, null, 2) + '\n');
} finally {
  await browser.close();
}
console.log('PASS source CSS spacing correction: 8 language/normal-width scenes; no real login or user-data access.');
