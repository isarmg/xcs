// Historical evidence probe, not current acceptance. See tests/evidence/README.md.
import {configureProbe, loadPlaywright, fixtureOrigin, requiredInput} from '../_support/runtime.mjs';
const historicalOrigin = fixtureOrigin();
const probe = await configureProbe(import.meta.url);
import {fileURLToPath} from 'node:url';
import {resolve} from 'node:path';
const {chromium}=await loadPlaywright();
import { readFile, writeFile, mkdir, readdir } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';
import { verifyPath } from './g4-reference.mjs';
const root = probe.root,
  base = historicalOrigin,
  output = probe.output;
const tokenCss = '\n:root { --zxs-compact-row-height:40px; --zxs-input-row-height:50px; }\n';
async function candidateCss(directory, prefix = '') {
  const paths = [];
  for (const entry of await readdir(directory, {
    withFileTypes: true
  })) {
    const path = prefix + '/' + entry.name;
    if (entry.isDirectory()) paths.push(...(await candidateCss(directory + '/' + entry.name, path)));else if (entry.isFile() && path.endsWith('.css')) paths.push(path);
  }
  return paths;
}
const expectedResources = [...(await candidateCss(root + '/web/dist')), '/pkg/zxs_bg.wasm'].sort();
const password = (await readFile(requiredInput('ZXS_BROWSER_PASSWORD_FILE'), 'utf8')).trim();
const seed = JSON.parse(await readFile(requiredInput('ZXS_BROWSER_SEED'), 'utf8')),
  fixtureFile = seed.files.find(f => f.path.endsWith('/说明.md'));
assert.ok(fixtureFile);
const fileId = fixtureFile.file_id,
  historyPath = `/v1/file-manager/files/${fileId}/history`;
const browser = await chromium.launch({
    headless: true,
    args: ['--no-sandbox']
  }),
  scenes = [],
  pathCache = new Map();
await mkdir(output, {
  recursive: true
});
const labels = {
  zh: {
    user: '用户名',
    password: '密码',
    login: '登录',
    logout: '退出',
    files: '文件',
    details: '详情',
    status: '状态',
    history: '历史记录',
    refresh: '刷新',
    account: '账号设置',
    save: '保存',
    saving: '保存中…',
    current: '当前密码',
    new: '新密码',
    confirm: '确认新密码',
    edit: '编辑备注',
    branch: '创建分支'
  },
  en: {
    user: 'Username',
    password: 'Password',
    login: 'Sign in',
    logout: 'Sign out',
    files: 'Files',
    details: 'Details',
    status: 'Status',
    history: 'History',
    refresh: 'Refresh',
    account: 'Account settings',
    save: 'Save',
    saving: 'Saving…',
    current: 'Current password',
    new: 'New password',
    confirm: 'Confirm new password',
    edit: 'Edit note',
    branch: 'Create branch'
  }
};
const digest = data => createHash('sha256').update(data).digest('hex');
function clean(e) {
  return String(e?.message || e).split(password).join('[redacted]');
}
async function scene(language, theme, width) {
  let geometry = {
      compact: 36,
      comfortable: 44,
      ratio: .4
    },
    tokenMode = false;
  const key = `${language}-${theme}-${width}`,
    t = labels[language],
    context = await browser.newContext({
      viewport: {
        width,
        height: 1100
      },
      colorScheme: theme
    }),
    page = await context.newPage();
  page.setDefaultTimeout(30000);
  const state = {
    language,
    theme,
    width,
    checks: [],
    snapshots: [],
    inventories: [],
    pageErrors: [],
    consoleErrors: [],
    assets: [],
    wasm: [],
    mockWrites: [],
    unexpectedWrites: [],
    loggedOut: false
  };
  let auth,
    loginInFlight = false,
    mockHistory = null,
    mockEvents = null,
    passwordPlan = null,
    holdGetPlan = null;
  const held = [],
    assetReads = [];
  await page.addInitScript(({
    theme
  }) => {
    localStorage.setItem('zxs-theme', theme);
    window.__zxsPaintBlobs = new Map();
    const old = URL.createObjectURL.bind(URL);
    URL.createObjectURL = blob => {
      const url = old(blob);
      window.__zxsPaintBlobs.set(url, blob);
      return url;
    };
    window.__zxsCspViolations = [];
    window.__zxsCsp = window.__zxsCspViolations;
    document.addEventListener('securitypolicyviolation', e => window.__zxsCspViolations.push({
      directive: e.effectiveDirective,
      blockedURI: e.blockedURI
    }));
  }, {
    theme
  });
  page.on('pageerror', e => state.pageErrors.push(e.message));
  page.on('console', e => {
    if (e.type() === 'error') state.consoleErrors.push(e.text());
  });
  page.on('request', r => {
    if (new URL(r.url()).pathname === '/v1/auth/login') loginInFlight = true;
  });
  for (const event of ['requestfinished', 'requestfailed']) page.on(event, r => {
    if (new URL(r.url()).pathname === '/v1/auth/login') loginInFlight = false;
  });
  page.on('response', r => {
    const path = new URL(r.url()).pathname;
    if (path === '/v1/auth/login' && r.ok()) assetReads.push(r.json().then(v => {
      auth = v;
    }).catch(e => {
      state.authReadError = clean(e);
    }));
    if (path.endsWith('.wasm')) state.wasm.push({
      status: r.status(),
      mime: r.headers()['content-type']
    });
    if (/\.(css|wasm)$/.test(path)) assetReads.push((async () => {
      try {
        const testTokenOverride = tokenMode && path === '/styles/components/geometry.css',
          remote = await r.body(),
          local = await readFile(root + '/web/dist' + path),
          expected = testTokenOverride ? Buffer.from(local.toString('utf8') + tokenCss) : local,
          sha256 = digest(remote);
        state.assets.push({
          path,
          status: r.status(),
          mime: r.headers()['content-type'],
          etag: r.headers().etag,
          sha256,
          localSha256: digest(local),
          expectedSha256: digest(expected),
          testTokenOverride,
          candidateSame: sha256 === digest(local),
          same: sha256 === digest(expected)
        });
      } catch (e) {
        state.assetReadErrors ??= [];
        state.assetReadErrors.push({
          path,
          error: clean(e)
        });
      }
    })());
  });
  await page.route('**/styles/components/geometry.css', async route => {
    if (!tokenMode) return route.continue();
    return route.fulfill({
      status: 200,
      contentType: 'text/css',
      body: (await readFile(root + '/web/dist/styles/components/geometry.css', 'utf8')) + tokenCss
    });
  });
  await page.route('**/v1/**', async route => {
    const r = route.request(),
      path = new URL(r.url()).pathname;
    if (r.method() === 'GET' && holdGetPlan && path === holdGetPlan.path) {
      const plan = holdGetPlan;
      holdGetPlan = null;
      plan.captured = true;
      await new Promise(resolve => {
        plan.release = resolve;
        held.push(resolve);
      });
      state.mockReads ??= [];
      state.mockReads.push({
        path,
        status: 401,
        delayedPreviousCredentialGeneration: true
      });
      return route.fulfill({
        status: 401,
        contentType: 'application/json',
        body: JSON.stringify({
          error: {
            code: 'ZXS_STALE_READ',
            message: 'Synthetic old-session response',
            retryable: false
          }
        })
      });
    }
    if (r.method() === 'GET' && path === '/v1/file-manager/history' && mockEvents) return route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(mockEvents)
    });
    if (r.method() === 'GET' && path === historyPath && mockHistory) return route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(mockHistory)
    });
    if (!['GET', 'HEAD'].includes(r.method()) && !['/v1/auth/login', '/v1/auth/logout'].includes(path)) {
      if (path === '/v1/auth/password' && passwordPlan) {
        const plan = passwordPlan;
        passwordPlan = null;
        const body = r.postDataJSON();
        assert.deepEqual(Object.keys(body).sort(), ['current_password', 'new_password']);
        assert.ok(body.current_password === 'synthetic-current-unused' && body.new_password === 'synthetic-new-unused', 'password probe must use synthetic strings only');
        state.mockWrites.push({
          path,
          method: r.method(),
          syntheticPasswordPayload: true
        });
        plan.captured = true;
        await new Promise(resolve => {
          plan.release = resolve;
          held.push(resolve);
        });
        return route.fulfill({
          status: 400,
          contentType: 'application/json',
          body: JSON.stringify({
            error: {
              code: 'ZXS_LAYOUT_MOCK',
              message: language === 'zh' ? '只读页面验收：请求已拦截，真实密码未修改。' : 'Read-only UI acceptance: request intercepted; the real password was not changed.',
              retryable: false
            }
          })
        });
      }
      state.unexpectedWrites.push({
        path,
        method: r.method()
      });
      return route.fulfill({
        status: 400,
        contentType: 'application/json',
        body: JSON.stringify({
          error: {
            code: 'ZXS_LAYOUT_GUARD',
            message: 'Unexpected write blocked',
            retryable: false
          }
        })
      });
    }
    return route.continue();
  });
  async function settle() {
    await page.evaluate(() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r))));
  }
  async function eventually(test, message) {
    const end = Date.now() + 10000;
    while (Date.now() < end) {
      if (await test()) return;
      await page.waitForTimeout(25);
    }
    throw new Error(message);
  }
  async function check(name, run) {
    try {
      await run();
      state.checks.push({
        name,
        passed: true
      });
      console.log(JSON.stringify({
        scene: key,
        check: name,
        passed: true
      }));
    } catch (e) {
      state.checks.push({
        name,
        passed: false,
        failure: clean(e)
      });
      throw e;
    }
  }
  const menu = name => page.locator('.zxs-header-navigation').getByRole('button', {
      name,
      exact: true
    }),
    shot = name => page.screenshot({
      path: `${output}/${name}-${key}.png`,
      fullPage: true
    });
  async function getReal(path) {
    const r = await context.request.get(base + path, {
      headers: auth?.token ? {
        Authorization: `Bearer ${auth.token}`
      } : {}
    });
    assert.equal(r.status(), 200);
    return r.json();
  }
  async function loginAppearance() {
    const appearance = await page.locator('.zxs-auth-card').evaluate(card => {
      const form = card.querySelector(':scope > form');
      const scope = card.closest('.zxs-auth-shell');
      const scopeStyle = getComputedStyle(scope);
      const cardRect = card.getBoundingClientRect();
      const formRect = form.getBoundingClientRect();
      const controls = [...form.querySelectorAll('input,button,.zxs-error')].map(element => {
        const bounds = element.getBoundingClientRect();
        return {
          tag: element.tagName,
          top: bounds.top,
          bottom: bounds.bottom,
          left: bounds.left,
          right: bounds.right
        };
      });
      return {
        cardWidth: cardRect.width,
        cardHeight: cardRect.height,
        formHeight: formRect.height,
        cardTop: cardRect.top,
        cardBottom: cardRect.bottom,
        cardLeft: cardRect.left,
        cardRight: cardRect.right,
        formBottom: formRect.bottom,
        clientHeight: form.clientHeight,
        scrollHeight: form.scrollHeight,
        formFont: parseFloat(getComputedStyle(form).fontSize),
        textSizes: [...form.querySelectorAll('h1,label,span,input,button,.zxs-error,.zxs-request-id,code')].map(element => parseFloat(getComputedStyle(element).fontSize)),
        controls,
        outerControlFonts: [...scope.querySelectorAll('.zxs-login-controls button')].map(button => getComputedStyle(button).fontSize),
        tokens: ['control', 'panel', 'notification', 'dialog'].map(name => scopeStyle.getPropertyValue(`--zxs-contour-${name}`).trim()),
        cardExtent: getComputedStyle(card).getPropertyValue('--zxs-contour-extent').trim(),
        documentWidth: document.documentElement.scrollWidth,
        viewport: innerWidth
      };
    });
    state.loginMeasurements ??= [];
    state.loginMeasurements.push(appearance);
    const expected = Math.max(14, appearance.cardWidth / 18);
    assert.ok(Math.abs(appearance.formFont - expected) <= 0.02, 'login form should preserve its previous size as cardWidth / 18');
    assert.ok(appearance.textSizes.every(size => Math.abs(size - expected) <= 0.02), 'all login fields, submit, and errors should inherit the original form font');
    assert.ok(appearance.outerControlFonts.every(size => size === '14px'), 'outer language/theme controls should keep their own 14px size');
    assert.deepEqual(appearance.tokens, ['27px', '42px', '48px', '54px']);
    assert.equal(appearance.cardExtent, '48px', 'login card should retain its original contour');
    assert.ok(appearance.formBottom <= appearance.cardBottom + 2, 'login form should fit its card without clipping');
    assert.ok(appearance.scrollHeight <= appearance.clientHeight + 1, 'login form content must not be clipped vertically');
    assert.ok(appearance.controls.every(control => control.top >= appearance.cardTop - 1 && control.bottom <= appearance.cardBottom + 1 && control.left >= appearance.cardLeft - 1 && control.right <= appearance.cardRight + 1), 'login controls and errors should stay inside the card');
    assert.ok(appearance.documentWidth <= appearance.viewport, 'login should not overflow horizontally');
    return appearance;
  }
  async function snapshot(name, {
    authPage = false
  } = {}) {
    await page.evaluate(() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r))));
    const data = await page.evaluate(async ({
      authPage,
      geometry
    }) => {
      const vis = e => {
        const b = e.getBoundingClientRect(),
          s = getComputedStyle(e);
        return b.width > 1 && b.height > 1 && s.display !== 'none' && s.visibility !== 'hidden' && (!e.closest('dialog') || e.closest('dialog').open);
      };
      const nodes = [...document.querySelectorAll(authPage ? '.zxs-auth-shell *' : '.zxs-app-shell *,dialog *, .zxs-select-panel:popover-open *')].filter(e => vis(e) && ([...e.childNodes].some(n => n.nodeType === 3 && n.textContent.trim()) || e.matches('input,textarea')));
      const typography = nodes.map(e => {
        const s = getComputedStyle(e),
          b = e.getBoundingClientRect();
        let textBounds = null;
        if (!e.matches('input,textarea')) {
          const r = document.createRange();
          r.selectNodeContents(e);
          const t = r.getBoundingClientRect();
          textBounds = {
            top: t.top - b.top,
            bottom: b.bottom - t.bottom,
            height: t.height,
            width: t.width
          };
        }
        return {
          textBounds,
          tag: e.tagName,
          cls: String(e.className),
          text: e.matches('input,textarea') ? '[input]' : e.textContent.trim().slice(0, 96),
          font: parseFloat(s.fontSize),
          lineHeight: s.lineHeight,
          height: b.height,
          width: b.width,
          extent: parseFloat(s.getPropertyValue('--zxs-contour-extent')) || 0,
          padding: {
            left: s.paddingLeft,
            right: s.paddingRight,
            top: s.paddingTop,
            bottom: s.paddingBottom
          },
          border: [s.borderTopWidth, s.borderRightWidth, s.borderBottomWidth, s.borderLeftWidth],
          align: s.alignItems,
          verticalAlign: s.verticalAlign,
          expectedFont: authPage ? null : e.matches('input.zxs-input:not(.zxs-input--compact), textarea.zxs-input:not(.zxs-input--compact)') ? geometry.comfortable * .4 : geometry.compact * .4,
          scrollWidth: e.scrollWidth,
          clientWidth: e.clientWidth,
          scrollHeight: e.scrollHeight,
          clientHeight: e.clientHeight
        };
      });
      const contours = [];
      for (const e of [...document.querySelectorAll('[data-zxs-contour-state]')].filter(vis)) {
        const s = getComputedStyle(e),
          b = e.getBoundingClientRect(),
          extent = parseFloat(s.getPropertyValue('--zxs-contour-extent'));
        if (!extent) continue;
        let path, source, sourceSvg;
        const fg = e.querySelector(':scope > [data-zxs-contour-decoration=foreground] path'),
          decor = e.querySelector(':scope > [data-zxs-contour-decoration=""]'),
          clip = (decor ? getComputedStyle(decor).clipPath : s.clipPath).match(/^path\("(.*)"\)$/);
        if (clip) {
          path = clip[1];
          source = 'actual computed clip-path';
        } else if (fg) {
          path = fg.getAttribute('d');
          source = 'actual foreground SVG';
        }
        if (!path) for (const image of [s.borderImageSource, s.backgroundImage, s.maskImage]) {
          for (const match of image.matchAll(/url\("?([^"\)]+)"?\)/g)) {
            const blob = window.__zxsPaintBlobs.get(match[1]);
            if (!blob) continue;
            const doc = new DOMParser().parseFromString(await blob.text(), 'image/svg+xml');
            path = (doc.querySelector('clipPath#inside>path') || doc.querySelector('path'))?.getAttribute('d');
            if (path) {
              source = 'actual emitted SVG Blob';
              const svg = doc.documentElement;
              sourceSvg = {
                width: parseFloat(svg.getAttribute('width')),
                height: parseFloat(svg.getAttribute('height'))
              };
              break;
            }
          }
          if (path) break;
        }
        ;
        const fs = fg && getComputedStyle(fg);
        contours.push({
          id: `${e.tagName}.${String(e.className)}`,
          width: b.width,
          height: b.height,
          extent,
          path,
          source,
          sourceSvg,
          state: e.dataset.zxsContourState,
          reason: e.dataset.zxsContourReason,
          borderToken: s.getPropertyValue('--zxs-block-border').trim(),
          foreground: fs ? {
            fill: fs.fill,
            fillRule: fs.fillRule,
            stroke: fs.stroke
          } : null,
          borderImage: {
            slice: s.borderImageSlice,
            width: s.borderImageWidth,
            outset: s.borderImageOutset
          }
        });
      }
      const headerControls = [...document.querySelectorAll('.zxs-header-actions button')].filter(vis).map(e => {
        const s = getComputedStyle(e),
          b = e.getBoundingClientRect(),
          svg = e.querySelector('svg')?.getBoundingClientRect();
        return {
          label: e.getAttribute('aria-label'),
          height: b.height,
          font: parseFloat(s.fontSize),
          extent: parseFloat(s.getPropertyValue('--zxs-contour-extent')),
          icon: svg ? {
            width: svg.width,
            height: svg.height
          } : null
        };
      });
      return {
        headerControls,
        runtime: document.documentElement.dataset.zxsRuntime,
        ready: document.documentElement.dataset.zxsReady,
        theme: document.documentElement.dataset.zxsTheme,
        viewport: innerWidth,
        documentWidth: document.documentElement.scrollWidth,
        typography,
        contours,
        csp: window.__zxsCspViolations || []
      };
    }, {
      authPage,
      geometry
    });
    for (const item of data.contours) {
      assert.ok(item.path, `${name}: missing painted path ${item.id} (${item.state}/${item.reason})`);
      const expected = Math.min(item.extent, item.height / 2, item.width / 2),
        cacheKey = item.path + '@' + expected;
      let g = pathCache.get(cacheKey);
      if (!g) {
        g = verifyPath(item.path, expected);
        pathCache.set(cacheKey, g);
      }
      item.geometry = g;
      if (!authPage) {
        assert.ok(Math.abs(item.extent - geometry.compact / 2) < .001 || Math.abs(item.extent - geometry.comfortable / 2) < .001, `${name}: stale extent ${item.id}=${item.extent}`);
        if (Math.abs(item.height - geometry.compact) < .02 || Math.abs(item.height - geometry.comfortable) < .02) {
          assert.ok(Math.abs(item.extent - item.height / 2) < .02, `${name}: single row upper/lower corners must meet halfway`);
          item.effectiveExtent = expected;
          item.sideStraightLength = item.height - 2 * expected;
          if (Math.abs(g.height - item.height) > .001) {
            assert.ok(Math.abs(g.height - (2 * expected + 1)) < .001, JSON.stringify({
              id: item.id,
              height: item.height,
              extent: item.extent,
              pathHeight: g.height,
              pathWidth: g.width,
              source: item.source,
              svg: item.sourceSvg,
              borderImage: item.borderImage
            }));
            assert.ok(item.sourceSvg);
            const margin = (item.sourceSvg.height - g.height) / 2;
            const slice = parseFloat(item.borderImage.slice) * (item.borderImage.slice.includes('%') ? item.sourceSvg.height / 100 : 1);
            assert.ok(Math.abs(parseFloat(item.borderImage.width) - (expected + margin)) < .001);
            assert.ok(Math.abs(parseFloat(item.borderImage.outset) - margin) < .001);
            assert.ok(Math.abs(slice - (expected + margin)) < .001);
            item.nineSliceMapping = {
              sourceMiddleHeight: 1,
              targetMiddleHeight: item.height - 2 * expected,
              preservesPhysicalCornerExtent: true
            };
          }
        }
      }
      if (item.foreground) {
        assert.equal(item.foreground.fillRule, 'evenodd');
        assert.equal(item.foreground.stroke, 'none');
        assert.notEqual(item.foreground.fill, 'rgb(0, 0, 0)');
      }
      delete item.path;
    }
    state.snapshots.push({
      name,
      ...data
    });
    await writeFile(`${output}/inventory-${name}-${key}.json`, JSON.stringify(data, null, 2) + '\n');
    assert.equal(data.runtime, 'zxs-web');
    assert.equal(data.ready, 'true');
    assert.equal(data.theme, theme);
    assert.ok(data.documentWidth <= data.viewport, `${name}: horizontal document overflow ${data.documentWidth}>${data.viewport}`);
    assert.deepEqual(data.csp, []);
    if (!authPage) {
      for (const c of data.headerControls) {
        assert.equal(c.height, geometry.comfortable);
        assert.ok(Math.abs(c.font - geometry.comfortable * .4) < .02);
        assert.ok(Math.abs(c.extent - geometry.comfortable / 2) < .02);
        if (c.icon) {
          assert.ok(Math.abs(c.icon.width - geometry.comfortable * .4) < .02);
          assert.ok(Math.abs(c.icon.height - geometry.comfortable * .4) < .02);
        }
      }
      const stale = data.typography.filter(i => Math.abs(i.font - i.expectedFont) > .02);
      assert.deepEqual(stale, [], `${name}: visible text must use compact14.4 or input17.6`);
    }
    return data;
  }
  async function statusLayout(name) {
    await settle();
    await page.waitForFunction(() => [...document.querySelectorAll('.zxs-metric-chart>svg')].every(s => {
      const b = s.parentElement.getBoundingClientRect(),
        v = s.viewBox.baseVal;
      return Math.abs(v.width - b.width) < .5 && Math.abs(v.height - b.height) < .5;
    }));
    const m = await page.locator('.zxs-metric-grid').evaluate(grid => {
      const g = getComputedStyle(grid);
      return {
        viewport: innerWidth,
        document: document.documentElement.scrollWidth,
        columns: g.gridTemplateColumns.split(/\s+/).length,
        cards: [...grid.querySelectorAll('.zxs-metric-card')].map(card => {
          const c = card.getBoundingClientRect(),
            chart = card.querySelector('.zxs-metric-chart'),
            s = chart?.querySelector('svg'),
            b = chart?.getBoundingClientRect();
          const labels = s ? [...s.querySelectorAll('.zxs-chart-label')].map(e => {
            const r = e.getBoundingClientRect();
            return {
              text: e.textContent,
              x: r.left - c.left,
              y: r.top - c.top,
              width: r.width,
              height: r.height
            };
          }) : [];
          return {
            key: card.dataset.metric,
            x: c.x,
            y: c.y,
            width: c.width,
            height: c.height,
            font: parseFloat(getComputedStyle(card).fontSize),
            extent: parseFloat(getComputedStyle(card).getPropertyValue('--zxs-contour-extent')),
            scrollWidth: card.scrollWidth,
            clientWidth: card.clientWidth,
            chart: b ? {
              width: b.width,
              height: b.height,
              svgNS: s?.namespaceURI,
              gridFill: s?.querySelector('.zxs-chart-grid') ? getComputedStyle(s.querySelector('.zxs-chart-grid')).fill : null,
              gridD: s?.querySelector('.zxs-chart-grid')?.getAttribute('d'),
              paths: s ? [...s.querySelectorAll('.zxs-chart-line')].map(e => ({
                namespace: e.namespaceURI,
                d: e.getAttribute('d'),
                fill: getComputedStyle(e).fill,
                stroke: getComputedStyle(e).stroke
              })) : [],
              dots: s ? [...s.querySelectorAll('.zxs-chart-dot')].map(e => ({
                namespace: e.namespaceURI,
                cx: e.getAttribute('cx'),
                cy: e.getAttribute('cy'),
                fill: getComputedStyle(e).fill
              })) : [],
              labels
            } : null
          };
        })
      };
    });
    assert.equal(m.cards.length, 9);
    assert.equal(m.columns, m.viewport <= 768 ? 1 : 4);
    assert.ok(m.document <= m.viewport);
    state.layoutAttempts ??= [];
    state.layoutAttempts.push({
      name,
      ...m
    });
    const first = m.cards[0];
    for (const c of m.cards) {
      assert.ok(Math.abs(c.width - first.width) < .05);
      assert.ok(Math.abs(c.width / c.height - 1.5) < .01, `${name} ${c.key}: 3:2`);
      assert.ok(Math.abs(c.font - 14.4) < .02);
      assert.equal(c.extent, 18);
      assert.ok(c.scrollWidth <= c.clientWidth + 1);
      if (c.chart?.svgNS) {
        assert.equal(c.chart.svgNS, 'http://www.w3.org/2000/svg');
        assert.equal(c.chart.gridFill, 'none');
        assert.equal((c.chart.gridD.match(/M/g) || []).length, 2, 'both maximum and zero grid lines remain present');
        for (const p of c.chart.paths) {
          assert.equal(p.namespace, 'http://www.w3.org/2000/svg');
          assert.equal(p.fill, 'none');
          assert.notEqual(p.stroke, 'rgb(0, 0, 0)');
          assert.ok(!/NaN|Infinity/.test(p.d));
        }
        for (const d of c.chart.dots) {
          assert.equal(d.namespace, 'http://www.w3.org/2000/svg');
          assert.notEqual(d.fill, 'rgb(0, 0, 0)');
        }
        const texts = c.chart.labels;
        for (let i = 0; i < texts.length; i++) for (let j = i + 1; j < texts.length; j++) {
          const a = texts[i],
            b = texts[j],
            overlapX = Math.min(a.x + a.width, b.x + b.width) - Math.max(a.x, b.x),
            overlapY = Math.min(a.y + a.height, b.y + b.height) - Math.max(a.y, b.y);
          assert.ok(overlapX <= 1 || overlapY <= 1, `${name} ${c.key}: chart labels overlap ${a.text}/${b.text}`);
        }
      }
    }
    assert.ok(Math.abs(m.cards.at(-1).x - first.x) < .05);
    state.layouts ??= [];
    state.layouts.push({
      name,
      ...m
    });
    return m;
  }
  async function observe(name, expectedIds = null, expectedEdges = null, {
    checkRows = true
  } = {}) {
    const data = await page.evaluate(() => {
      const root = document.querySelector('.zxs-version-tree-canvas'),
        scroll = root.parentElement,
        svg = root.querySelector('svg.zxs-version-tree-links'),
        r = root.getBoundingClientRect();
      const rect = e => {
        const b = e.getBoundingClientRect();
        return {
          x: b.x - r.x,
          y: b.y - r.y,
          width: b.width,
          height: b.height,
          bottom: b.bottom - r.y,
          right: b.right - r.x
        };
      };
      const nodes = [...root.querySelectorAll('.zxs-version-node')].map(e => ({
        id: e.dataset.versionId,
        depth: Number(e.closest('[data-version-depth]').dataset.versionDepth),
        ...rect(e),
        children: [...e.children].filter(x => !x.hasAttribute('data-zxs-contour-decoration')).map(c => ({
          cls: c.className,
          tag: c.tagName,
          ...rect(c),
          font: parseFloat(getComputedStyle(c).fontSize),
          lineHeight: getComputedStyle(c).lineHeight
        })),
        buttons: [...e.querySelectorAll('button')].map(b => ({
          text: b.textContent.trim(),
          disabled: b.disabled,
          ...rect(b),
          font: parseFloat(getComputedStyle(b).fontSize)
        })),
        text: e.textContent,
        extent: parseFloat(getComputedStyle(e).getPropertyValue('--zxs-contour-extent')),
        border: getComputedStyle(e).borderTopWidth,
        current: e.querySelector('.zxs-version-current')?.textContent || null,
        currentInDate: !!e.querySelector('.zxs-version-date .zxs-version-current')
      }));
      const edges = [...svg.querySelectorAll('.zxs-version-tree-edge')].map(e => {
        const n = e.getTotalLength();
        return {
          parent: e.dataset.parentVersion,
          child: e.dataset.childVersion,
          namespace: e.namespaceURI,
          d: e.getAttribute('d'),
          fill: getComputedStyle(e).fill,
          stroke: getComputedStyle(e).stroke,
          marker: e.getAttribute('marker-end'),
          points: Array.from({
            length: 101
          }, (_, i) => {
            const p = e.getPointAtLength(n * i / 100);
            return {
              x: p.x,
              y: p.y
            };
          })
        };
      });
      const marker = svg.querySelector('marker'),
        arrow = marker.querySelector('path');
      return {
        viewport: innerWidth,
        document: document.documentElement.scrollWidth,
        canvas: {
          width: r.width,
          height: r.height
        },
        scroll: {
          clientWidth: scroll.clientWidth,
          scrollWidth: scroll.scrollWidth,
          overflowX: getComputedStyle(scroll).overflowX
        },
        nodes,
        edges,
        svgBounds: rect(svg),
        viewBox: svg.getAttribute('viewBox'),
        svgNS: svg.namespaceURI,
        markerNS: marker.namespaceURI,
        arrow: {
          namespace: arrow.namespaceURI,
          fill: getComputedStyle(arrow).fill,
          stroke: getComputedStyle(arrow).stroke,
          d: arrow.getAttribute('d'),
          refX: marker.getAttribute('refX')
        },
        csp: window.__zxsCsp
      };
    });
    if (expectedIds) assert.deepEqual(data.nodes.map(n => n.id).sort(), [...expectedIds].sort(), `${name}: API versions and UI nodes`);
    if (expectedEdges) assert.deepEqual(data.edges.map(e => `${e.parent}->${e.child}`).sort(), [...expectedEdges].sort(), `${name}: API ancestry and SVG arrows`);
    assert.ok(data.document <= data.viewport, `${name}: document horizontal overflow`);
    assert.ok(['auto', 'scroll'].includes(data.scroll.overflowX));
    assert.ok(Math.abs(data.svgBounds.width - data.canvas.width) < .1 && Math.abs(data.svgBounds.height - data.canvas.height) < .1, `${name}: SVG viewport must align with measured tree canvas`);
    const vb = data.viewBox.split(/\s+/).map(Number);
    assert.ok(Math.abs(vb[2] - data.canvas.width) < .1 && Math.abs(vb[3] - data.canvas.height) < .1, `${name}: SVG viewBox must match physical tree dimensions`);
    assert.equal(data.svgNS, 'http://www.w3.org/2000/svg');
    assert.equal(data.markerNS, data.svgNS);
    assert.equal(data.arrow.namespace, data.svgNS);
    assert.equal(data.arrow.stroke, 'none');
    assert.notEqual(data.arrow.fill, 'rgb(0, 0, 0)');
    assert.equal(data.arrow.refX, '8');
    assert.deepEqual(data.csp, []);
    for (const n of data.nodes) {
      assert.equal(n.children.length, 3, `${name}: node must have three logical rows`);
      assert.ok(n.children[0].cls.includes('zxs-version-date'));
      assert.ok(n.children[1].cls.includes('zxs-version-note-row'));
      assert.ok(n.children[2].cls.includes('zxs-version-actions'));
      assert.equal(n.extent, geometry.compact / 2);
      assert.equal(n.border, '1px');
      if (checkRows) {
        assert.equal(n.buttons.length, 2);
        assert.deepEqual(n.buttons.map(b => b.text), [t.edit, t.branch]);
        for (const row of n.children) {
          assert.ok(row.height >= geometry.compact - .1);
          assert.ok(Math.abs(row.font - geometry.compact * .4) < .02);
        }
      }
      if (n.current) assert.ok(n.currentInDate);
    }
    for (const edge of data.edges) {
      assert.equal(edge.namespace, data.svgNS);
      assert.equal(edge.fill, 'none');
      assert.notEqual(edge.stroke, 'none');
      assert.notEqual(edge.stroke, 'rgb(0, 0, 0)');
      assert.equal(edge.marker, 'url(#zxs-version-arrow)');
      assert.ok(!/NaN|Infinity/.test(edge.d));
      const parent = data.nodes.find(n => n.id === edge.parent),
        child = data.nodes.find(n => n.id === edge.child),
        a = edge.points[0],
        b = edge.points.at(-1);
      assert.ok(Math.abs(a.x - (parent.x + parent.width / 2)) < .02 && Math.abs(a.y - parent.y) < .02, `${name}: parent-top endpoint, arrow travels upward`);
      assert.ok(Math.abs(b.x - (child.x + child.width / 2)) < .02 && Math.abs(b.y - (child.bottom + 3)) < .02, `${name}: child-bottom arrow endpoint, newer child above`);
      assert.ok(parent.depth > child.depth && a.y > b.y, `${name}: old parent must be below new child`);
      for (let i = 1; i < edge.points.length; i++) assert.ok(edge.points[i].y <= edge.points[i - 1].y + .02, `${name}: upward path should not reverse vertically`);
      for (const point of edge.points.slice(1, -1)) {
        for (const n of data.nodes) assert.ok(!(point.x > n.x + .5 && point.x < n.right - .5 && point.y > n.y + .5 && point.y < n.bottom - .5), `${name}: arrow crosses node ${n.id}`);
      }
    }
    state.inventories.push({
      name,
      ...data
    });
    await writeFile(`${output}/inventory-${name}-${key}.json`, JSON.stringify(data, null, 2) + '\n');
    return data;
  }
  async function stableObserve(name, ids, edges, options) {
    let failure;
    for (let i = 0; i < 40; i++) {
      try {
        return await observe(name, ids, edges, options);
      } catch (e) {
        failure = e;
        await page.waitForTimeout(50);
      }
    }
    throw failure;
  }
  async function flexGaps(name) {
    const selectors = ['.zxs-page-header', '.zxs-header-brand-navigation', '.zxs-header-navigation', '.zxs-header-actions', '.zxs-breadcrumb', '.zxs-file-toolbar', '.zxs-entry-navigation', '.zxs-file-heading', '.zxs-entry-actions', '.zxs-version-actions', '.zxs-account-form>.zxs-actions', '.zxs-monitor-toolbar', '.zxs-history-toolbar'];
    const records = await page.evaluate(selectors => selectors.flatMap(selector => [...document.querySelectorAll(selector)].filter(e => e.getBoundingClientRect().width > 0).map(e => {
      const s = getComputedStyle(e),
        children = [...e.children].filter(c => {
          const b = c.getBoundingClientRect();
          return b.width > 0 && b.height > 0 && !c.hasAttribute('data-zxs-contour-decoration') && getComputedStyle(c).position !== 'absolute';
        }).map(c => {
          const b = c.getBoundingClientRect();
          return {
            top: b.top,
            bottom: b.bottom,
            left: b.left,
            right: b.right,
            height: b.height
          };
        }).sort((a, b) => a.top - b.top || a.left - b.left);
      const lines = [];
      for (const c of children) {
        let line = lines.find(l => Math.min(l.bottom, c.bottom) - Math.max(l.top, c.top) > 1);
        if (line) {
          line.top = Math.min(line.top, c.top);
          line.bottom = Math.max(line.bottom, c.bottom);
          line.children.push(c);
        } else lines.push({
          top: c.top,
          bottom: c.bottom,
          children: [c]
        });
      }
      lines.sort((a, b) => a.top - b.top);
      return {
        selector,
        display: s.display,
        wrap: s.flexWrap,
        rowGap: s.rowGap,
        columnGap: s.columnGap,
        lines,
        actualRowGaps: lines.slice(1).map((l, i) => l.top - lines[i].bottom)
      };
    })), selectors);
    for (const r of records) {
      assert.equal(parseFloat(r.columnGap), 0, name + ' ' + r.selector + ' column gap');
      if (r.wrap !== 'wrap') {
        assert.equal(r.wrap, 'nowrap');
        continue;
      }
      assert.equal(parseFloat(r.rowGap), 0, name + ' ' + r.selector + ' unframed flow has no added row gap');
      for (const g of r.actualRowGaps) assert.ok(Math.abs(g) < .15, `${name} ${r.selector} unframed wrapped rows must meet without added spacing: ${g}`);
    }
    state.flexGaps ??= [];
    state.flexGaps.push({
      name,
      records
    });
    return records;
  }
  async function blockGaps(name, selectors) {
    const records = await page.evaluate(selectors => {
      const visible = e => {
        const b = e.getBoundingClientRect(), s = getComputedStyle(e);
        return b.width > 0 && b.height > 0 && s.display !== 'none' && s.visibility !== 'hidden'
          && !['absolute', 'fixed'].includes(s.position) && !e.hasAttribute('data-zxs-contour-decoration');
      };
      const rect = e => {
        const b = e.getBoundingClientRect();
        return { top: b.top, bottom: b.bottom, left: b.left, right: b.right };
      };
      const painted = '.zxs-frame,.zxs-input:not(.zxs-input--underline)';
      return selectors.flatMap(selector => [...document.querySelectorAll(selector)].filter(visible).map(e => {
        const children = [...e.children].filter(visible).map(c => ({
          cls: String(c.className),
          ...rect(c),
          marginTop: parseFloat(getComputedStyle(c).marginTop),
          surfacePair: c.matches(':is(.zxs-frame,.zxs-surface,.zxs-surface-end) + :is(.zxs-frame,.zxs-surface,.zxs-surface-start)'),
          // On phones a full explanatory row already separates the branch select
          // from the list; adding another half-row would count that clearance twice.
          phoneExplanation: matchMedia('(max-width:36rem)').matches
            && c.matches('.zxs-file-toolbar:has(> .zxs-select) + .zxs-surface'),
          paints: [...(c.matches(painted) ? [c] : []), ...c.querySelectorAll(painted)].filter(visible).map(rect)
        }));
        return {
          selector,
          cls: String(e.className),
          gap: getComputedStyle(e).rowGap,
          children,
          pairs: children.slice(1).map((c, i) => {
            const previous = children[i];
            const gaps = previous.paints.flatMap(a => c.paints.filter(b =>
              Math.min(a.right, b.right) - Math.max(a.left, b.left) > 1
            ).map(b => b.top - a.bottom));
            return {
              before: previous.cls,
              after: c.cls,
              actual: c.top - previous.bottom,
              marginTop: c.marginTop,
              needsHalfRow: c.surfacePair && !c.phoneExplanation,
              phoneExplanation: c.phoneExplanation,
              nearestPaintGap: gaps.length ? Math.min(...gaps) : null
            };
          })
        };
      }));
    }, selectors);
    for (const r of records) {
      assert.equal(parseFloat(r.gap), 0, `${name} ${r.selector}: ordinary stacks have no global gap`);
      for (const pair of r.pairs) {
        const expected = pair.needsHalfRow ? geometry.compact / 2 : 0;
        assert.ok(Math.abs(pair.actual - expected) < .2, `${name} ${r.selector}: physical gap ${pair.actual}, expected ${expected} between ${pair.before} and ${pair.after}`);
        assert.ok(Math.abs(pair.marginTop - expected) < .2, `${name} ${r.selector}: only adjacent painted boundaries receive extra margin`);
        if (pair.nearestPaintGap !== null) assert.ok(pair.nearestPaintGap >= geometry.compact / 2 - .2,
          `${name} ${r.selector}: overlapping horizontal painted boundaries need half-row clearance, got ${pair.nearestPaintGap}`);
      }
    }
    state.blockGaps ??= [];
    state.blockGaps.push({ name, records });
    return records;
  }
  async function rowRecords(name, selectors, {
    minHeight = 36,
    padding = 9.8,
    exact = false,
    marginZero = true
  } = {}) {
    const records = await page.evaluate(selectors => selectors.flatMap(selector => [...document.querySelectorAll(selector)].filter(e => {
      const b = e.getBoundingClientRect(),
        s = getComputedStyle(e);
      return b.width > 0 && b.height > 0 && s.position !== 'absolute' && s.visibility !== 'hidden';
    }).map(e => {
      const b = e.getBoundingClientRect(),
        s = getComputedStyle(e);
      const range = document.createRange();
      range.selectNodeContents(e);
      const text = range.getBoundingClientRect();
      return {
        selector,
        cls: String(e.className),
        tag: e.tagName,
        text: e.matches('input') ? '[input]' : e.textContent.slice(0, 100),
        height: b.height,
        width: b.width,
        top: b.top,
        bottom: b.bottom,
        left: b.left,
        right: b.right,
        font: parseFloat(s.fontSize),
        lineHeight: s.lineHeight,
        padding: {
          left: parseFloat(s.paddingLeft),
          right: parseFloat(s.paddingRight),
          top: parseFloat(s.paddingTop),
          bottom: parseFloat(s.paddingBottom)
        },
        margin: {
          top: parseFloat(s.marginTop),
          bottom: parseFloat(s.marginBottom)
        },
        boxSizing: s.boxSizing,
        display: s.display,
        minHeight: parseFloat(s.minHeight) || 0,
        border: [s.borderTopWidth, s.borderRightWidth, s.borderBottomWidth, s.borderLeftWidth],
        textBox: {
          top: text.top - b.top,
          bottom: b.bottom - text.bottom,
          height: text.height
        }
      };
    })), selectors);
    assert.ok(records.length > 0, `${name}: selectors must match actual page rows`);
    for (const r of records) {
      assert.ok(r.height >= minHeight - .05, `${name} ${r.selector} minimum row36, got ${r.height}`);
      if (exact) assert.ok(Math.abs(r.height - minHeight) < .05, `${name} ${r.selector} must be ${minHeight}px`);
      assert.ok(Math.abs(r.font - 14.4) < .02);
      assert.ok(Math.abs(r.padding.left - padding) < .05, `${name} ${r.selector}: leading padding ${r.padding.left} != ${padding}`);
      if (marginZero) {
        assert.equal(r.margin.top, 0);
        assert.equal(r.margin.bottom, 0);
      }
      if (['DT', 'DD'].includes(r.tag)) assert.ok(Math.abs(r.textBox.top + r.textBox.height / 2 - r.height / 2) <= 1.5, `${name} ${r.tag}: metadata glyph group must be vertically centered even when value wraps`);
    }
    state.rowRecords ??= [];
    state.rowRecords.push({
      name,
      records
    });
    return records;
  }
  async function detailGeometry(name) {
    const data = await page.locator('.zxs-entry-content').evaluate(panel => {
      const s = getComputedStyle(panel),
        b = panel.getBoundingClientRect(),
        dl = panel.querySelector('.zxs-file-properties');
      return {
        padding: [s.paddingTop, s.paddingRight, s.paddingBottom, s.paddingLeft],
        border: [s.borderTopWidth, s.borderRightWidth, s.borderBottomWidth, s.borderLeftWidth],
        extent: parseFloat(s.getPropertyValue('--zxs-contour-extent')),
        font: parseFloat(s.fontSize),
        propertiesGap: getComputedStyle(dl).rowGap,
        sections: [...panel.querySelectorAll('.zxs-entry-section')].map(e => ({
          cls: e.className,
          gap: getComputedStyle(e).rowGap
        })),
        firstLabelOffset: dl.querySelector('dt').getBoundingClientRect().left - b.left + parseFloat(getComputedStyle(dl.querySelector('dt')).paddingLeft)
      };
    });
    assert.deepEqual(data.padding, ['0px', '0px', '0px', '0px']);
    assert.deepEqual(data.border, ['1px', '1px', '1px', '1px']);
    assert.equal(data.extent, 18);
    assert.equal(parseFloat(data.propertiesGap), 0);
    assert.ok(Math.abs(data.firstLabelOffset - 10.8) < .05);
    for (const section of data.sections) assert.equal(parseFloat(section.gap), 0, name + ': section labels already provide row clearance');
    state.detailGeometry ??= [];
    state.detailGeometry.push({
      name,
      ...data
    });
    await rowRecords(name, ['.zxs-entry-content > .zxs-file-heading > h2', '.zxs-entry-content > .zxs-file-heading .zxs-entry-actions > .zxs-button', '.zxs-version-date', '.zxs-version-note-row:not(:has(input))', '.zxs-version-actions .zxs-button', '.zxs-file-properties dt', '.zxs-file-properties dd', '.zxs-entry-section > h3', '.zxs-entry-section > p', '.zxs-entry-section .zxs-select-label', '.zxs-entry-content > .zxs-file-toolbar > button', '.zxs-entry-content .zxs-inline-edit > button', '.zxs-entry-content .zxs-entry-section .zxs-inline-branch > button']);
    if (await page.locator('.zxs-entry-section .zxs-select-trigger').count()) await rowRecords(name + '-branch-trigger', ['.zxs-entry-section .zxs-select-trigger'], {
      exact: true
    });
    await blockGaps(name, ['.zxs-entry-page', '.zxs-entry-content', '.zxs-entry-version-history']);
  }
  async function historyGeometry(name) {
    await page.locator('.zxs-history-list-panel').waitFor();
    const data = await page.locator('.zxs-history-list-panel').evaluate(panel => {
      const b = panel.getBoundingClientRect(),
        s = getComputedStyle(panel),
        list = panel.querySelector('ul');
      return {
        viewport: innerWidth,
        document: document.documentElement.scrollWidth,
        panel: {
          padding: [s.paddingTop, s.paddingRight, s.paddingBottom, s.paddingLeft],
          border: [s.borderTopWidth, s.borderRightWidth, s.borderBottomWidth, s.borderLeftWidth],
          extent: parseFloat(s.getPropertyValue('--zxs-contour-extent'))
        },
        list: {
          gap: getComputedStyle(list).rowGap,
          padding: getComputedStyle(list).padding
        },
        rows: [...list.children].map(row => {
          const rs = getComputedStyle(row);
          const cells = [...row.children].map(c => {
            const r = c.getBoundingClientRect(),
              cs = getComputedStyle(c);
            return {
              cls: c.className,
              font: parseFloat(cs.fontSize),
              top: r.top,
              bottom: r.bottom,
              left: r.left,
              right: r.right,
              height: r.height,
              paddingLeft: parseFloat(cs.paddingLeft),
              align: cs.alignItems,
              text: c.textContent
            };
          });
          const lines = [];
          for (const c of cells) {
            const line = lines.find(l => Math.min(l.bottom, c.bottom) - Math.max(l.top, c.top) > 1);
            if (line) {
              line.top = Math.min(line.top, c.top);
              line.bottom = Math.max(line.bottom, c.bottom);
            } else lines.push({
              top: c.top,
              bottom: c.bottom
            });
          }
          lines.sort((a, b) => a.top - b.top);
          return {
            cells,
            rowGap: parseFloat(rs.rowGap),
            columnGap: parseFloat(rs.columnGap),
            padding: rs.padding,
            borderBottom: rs.borderBottomWidth,
            lines,
            actualWrapGaps: lines.slice(1).map((l, i) => l.top - lines[i].bottom),
            leadingOffset: cells.length ? cells[0].left - b.left + cells[0].paddingLeft : null
          };
        })
      };
    });
    assert.deepEqual(data.panel.padding, ['0px', '0px', '0px', '0px']);
    assert.deepEqual(data.panel.border, ['1px', '1px', '1px', '1px']);
    assert.equal(data.panel.extent, 18);
    assert.equal(parseFloat(data.list.gap), 0);
    assert.ok(data.document <= data.viewport);
    for (const row of data.rows) {
      assert.equal(row.cells.length, 4);
      assert.equal(row.rowGap, 0);
      assert.equal(row.columnGap, 0);
      assert.equal(row.padding, '0px');
      assert.ok(Math.abs(row.leadingOffset - 10.8) < .05);
      for (const cell of row.cells) {
        assert.equal(cell.font, 14.4);
        assert.ok(cell.height >= 35.95);
        assert.ok(Math.abs(cell.paddingLeft - 9.8) < .05);
        assert.equal(cell.align, 'center');
      }
      for (const gap of row.actualWrapGaps) assert.ok(Math.abs(gap) < .1, name + ': unframed history cells must not gain half-row spacing');
    }
    if (data.rows.length) assert.equal(data.rows.at(-1).borderBottom, '0px');
    state.historyGeometry ??= [];
    state.historyGeometry.push({
      name,
      ...data
    });
    await rowRecords(name + '-headers', ['.zxs-history-page > p', '.zxs-history-toolbar .zxs-select-label'], {
      padding: 10.8
    });
    await rowRecords(name + '-trigger', ['.zxs-history-toolbar .zxs-select-trigger'], {
      padding: 9.8,
      exact: true
    });
    await blockGaps(name, ['.zxs-history-page']);
    return data;
  }
  function version(id, parents, day, note = '', current = false) {
    return {
      version_id: id,
      file_id: fileId,
      source_version_ids: parents,
      created_at: `2026-01-${String(day).padStart(2, '0')}T01:02:03Z`,
      note,
      current
    };
  }
  const nodeById = id => page.locator(`.zxs-version-node[data-version-id=${JSON.stringify(id)}]`);
  async function refreshDetails() {
    const response = page.waitForResponse(r => new URL(r.url()).pathname === historyPath && r.ok());
    await page.getByRole('button', {
      name: t.refresh,
      exact: true
    }).evaluate(e => e.click());
    await response;
    await settle();
  }
  async function assertReverse(name, records) {
    const expectedEdges = records.flatMap(v => (v.source_version_ids || []).filter(p => records.some(r => r.version_id === p) && p !== v.version_id).map(p => `${p}->${v.version_id}`));
    const data = await stableObserve(name, records.map(v => v.version_id), expectedEdges);
    const byId = new Map(data.nodes.map(n => [n.id, n]));
    const parents = new Set(expectedEdges.map(e => e.split('->')[0]));
    for (const v of records) {
      if (!parents.has(v.version_id)) assert.equal(byId.get(v.version_id).depth, 0, `${name}: every terminal version belongs in latest top layer`);
      for (const source of v.source_version_ids || []) {
        if (byId.has(source)) assert.ok(byId.get(source).depth > byId.get(v.version_id).depth);
      }
    }
    const top = data.nodes.filter(n => n.depth === 0).sort((a, b) => a.x - b.x).map(n => n.id);
    const wanted = records.filter(v => !parents.has(v.version_id)).sort((a, b) => Date.parse(b.created_at) - Date.parse(a.created_at) || a.version_id.localeCompare(b.version_id)).map(v => v.version_id);
    assert.deepEqual(top, wanted);
    state.reverseTopology ??= [];
    state.reverseTopology.push({
      name,
      topLayer: top,
      oldestDepth: Math.max(...data.nodes.map(n => n.depth)),
      allParentsBelowChildren: true,
      arrowsPointUp: true
    });
    return data;
  }
  async function sharedRoles(name) {
    await settle();
    const data = await page.evaluate(({
      geometry
    }) => {
      const visible = e => {
        const b = e.getBoundingClientRect(),
          s = getComputedStyle(e);
        return b.width > 1 && b.height > 1 && s.display !== 'none' && s.visibility !== 'hidden';
      };
      const box = e => {
        const b = e.getBoundingClientRect(),
          s = getComputedStyle(e);
        return {
          cls: String(e.className),
          height: b.height,
          width: b.width,
          font: parseFloat(s.fontSize),
          extent: parseFloat(s.getPropertyValue('--zxs-contour-extent')) || 0,
          border: parseFloat(s.borderLeftWidth),
          paddingLeft: parseFloat(s.paddingLeft),
          rowBorder: parseFloat(s.getPropertyValue('--zxs-row-border')) || 0,
          rowGap: parseFloat(s.rowGap),
          columnGap: parseFloat(s.columnGap)
        };
      };
      const choose = s => [...document.querySelectorAll(s)].filter(visible);
      return {
        missingPanels: choose('.zxs-app-shell .zxs-content-panel').filter(e => !e.classList.contains('zxs-frame')).map(box),
        missingStacks: choose('.zxs-app-shell .zxs-content-stack').filter(e => !e.classList.contains('zxs-stack')).map(box),
        missingCompacts: choose('.zxs-file-search input,.zxs-account-page input,.zxs-select-trigger').filter(e => !e.classList.contains('zxs-input--compact')).map(box),
        missingUnderlines: choose('input.zxs-inline-input,input.zxs-branch-name').filter(e => !e.classList.contains('zxs-input--underline')).map(box),
        frames: choose('.zxs-app-shell .zxs-frame').map(box),
        stacks: choose('.zxs-app-shell .zxs-stack').map(box),
        flows: choose('.zxs-app-shell .zxs-flow').map(box),
        rows: choose('.zxs-app-shell .zxs-row').map(box),
        inputs: choose('.zxs-app-shell .zxs-input').map(e => ({
          ...box(e),
          compact: e.classList.contains('zxs-input--compact'),
          underline: e.classList.contains('zxs-input--underline')
        })),
        noticeContainers: document.querySelectorAll('.zxs-app-shell .zxs-status').length,
        errorContainers: document.querySelectorAll('.zxs-app-shell .zxs-error-center').length,
        mainVisibleChildren: choose('.zxs-shell-main > *:not([data-zxs-contour-decoration])').map(box),
        pageWidth: document.documentElement.scrollWidth,
        viewport: innerWidth,
        geometry
      };
    }, {
      geometry
    });
    assert.deepEqual(data.missingPanels, [], name + ': every business panel composes shared frame');
    assert.deepEqual(data.missingStacks, [], name + ': every business page composes shared stack');
    assert.deepEqual(data.missingCompacts, [], name + ': compact fields are explicit');
    assert.deepEqual(data.missingUnderlines, [], name + ': inline input role is explicit');
    for (const frame of data.frames) {
      assert.equal(frame.extent, geometry.compact / 2, name + ': frame contour');
      assert.equal(frame.border, 1, name + ': frame border');
    }
    for (const stack of data.stacks) assert.equal(stack.rowGap, 0, name + ': default stack gap is zero');
    for (const flow of data.flows) {
      assert.equal(flow.rowGap, 0, name + ': unframed flow has no added row gap');
      assert.equal(flow.columnGap, 0, name + ': same-line gap');
    }
    for (const row of data.rows) {
      assert.ok(Math.abs(row.font - geometry.compact * .4) < .02, name + ': shared row font');
      assert.ok(row.height >= geometry.compact - .1, name + ': minimum row height');
    }
    for (const input of data.inputs) {
      const h = input.compact ? geometry.compact : geometry.comfortable;
      assert.ok(Math.abs(input.height - h) < .05, name + ': input row');
      assert.ok(Math.abs(input.font - h * .4) < .02, name + ': input font');
      if (input.underline) {
        assert.equal(input.extent, 0);
        assert.ok(Math.abs(input.paddingLeft - (h * .3 - input.rowBorder)) < .05, name + ': underline inset');
      } else {
        assert.equal(input.extent, h / 2);
        assert.ok(Math.abs(input.paddingLeft - (h * .3 - 1)) < .05, name + ': input inset');
      }
    }
    assert.equal(data.noticeContainers, 0, name + ': empty notice should not occupy a stack slot');
    assert.equal(data.errorContainers, 0, name + ': empty errors should not occupy a stack slot');
    assert.equal(data.mainVisibleChildren.length, 1, name + ': only the page occupies the main stack');
    assert.ok(data.pageWidth <= data.viewport, name + ': document width');
    state.roles ??= [];
    state.roles.push({
      name,
      ...data
    });
    await blockGaps(name + '-shared-stacks', ['.zxs-app-shell .zxs-stack']);
    return data;
  }
  async function ordinaryInput(name) {
    await page.evaluate(() => {
      const input = document.createElement('input');
      input.className = 'zxs-input';
      input.setAttribute('data-zxs-probe', 'ordinary-input');
      input.setAttribute('aria-label', 'Ordinary component geometry probe');
      input.value = 'component probe';
      document.querySelector('.zxs-shell-main > .zxs-content-stack').append(input);
    });
    try {
      await settle();
      await eventually(async () => (await page.locator('[data-zxs-probe=ordinary-input]').getAttribute('data-zxs-contour-state')) !== null, 'ordinary component receives the actual Rust contour');
      const value = await page.locator('[data-zxs-probe=ordinary-input]').evaluate(e => {
        const s = getComputedStyle(e);
        return {
          height: e.getBoundingClientRect().height,
          font: parseFloat(s.fontSize),
          padding: parseFloat(s.paddingLeft),
          extent: parseFloat(s.getPropertyValue('--zxs-contour-extent'))
        };
      });
      assert.equal(value.height, geometry.comfortable);
      assert.equal(value.font, geometry.comfortable * .4);
      assert.equal(value.extent, geometry.comfortable / 2);
      assert.ok(Math.abs(value.padding - (geometry.comfortable * .3 - 1)) < .05);
      state.ordinaryInputs ??= [];
      state.ordinaryInputs.push({
        name,
        ...value
      });
      await snapshot(name);
    } finally {
      await page.locator('[data-zxs-probe=ordinary-input]').evaluate(e => e.remove());
    }
  }
  async function inspectSelect(name, selector) {
    const trigger = page.locator(selector).first();
    await trigger.click();
    const panel = page.locator('.zxs-select-panel:popover-open');
    await panel.waitFor();
    await page.keyboard.press('ArrowDown');
    await settle();
    const options = await panel.locator('.zxs-select-option').evaluateAll(elements => elements.map(e => {
      const s = getComputedStyle(e);
      return { height: e.getBoundingClientRect().height, font: parseFloat(s.fontSize), paddingLeft: parseFloat(s.paddingLeft), extent: parseFloat(s.getPropertyValue('--zxs-contour-extent')) };
    }));
    assert.ok(options.length > 0);
    for (const option of options) {
      assert.equal(option.height, geometry.compact);
      assert.equal(option.font, geometry.compact * .4);
      assert.ok(Math.abs(option.paddingLeft - (geometry.compact * .3 - 1)) < .05);
      assert.equal(option.extent, 0);
    }
    state.selectOptions ??= [];
    state.selectOptions.push({ name, options });
    await sharedRoles(name);
    await snapshot(name);
    await page.keyboard.press('Escape');
    await panel.waitFor({ state: 'hidden' });
  }
  async function accountGeometry(name) {
    const data = await page.locator('.zxs-account-page').evaluate(page => {
      const panel = page.querySelector('.zxs-content-panel'),
        form = page.querySelector('form'),
        fields = page.querySelector('.zxs-account-fields');
      const box = e => {
        const b = e.getBoundingClientRect(),
          s = getComputedStyle(e);
        return {
          top: b.top,
          bottom: b.bottom,
          left: b.left,
          right: b.right,
          width: b.width,
          height: b.height,
          font: parseFloat(s.fontSize),
          extent: parseFloat(s.getPropertyValue('--zxs-contour-extent')),
          padLeft: parseFloat(s.paddingLeft),
          border: [s.borderTopWidth, s.borderRightWidth, s.borderBottomWidth, s.borderLeftWidth],
          display: s.display,
          readOnly: e.readOnly,
          disabled: e.disabled,
          autocomplete: e.autocomplete
        };
      };
      return {
        viewport: innerWidth,
        document: document.documentElement.scrollWidth,
        panel: box(panel),
        rootEm: parseFloat(getComputedStyle(document.documentElement).fontSize),
        formGap: getComputedStyle(form).rowGap,
        fieldGap: getComputedStyle(fields).rowGap,
        columnGap: getComputedStyle(fields).columnGap,
        labels: [...page.querySelectorAll('label')].map(label => ({
          span: box(label.querySelector('span')),
          input: box(label.querySelector('input'))
        })),
        save: box(page.querySelector('button')),
        error: page.querySelector('.zxs-error') ? box(page.querySelector('.zxs-error')) : null
      };
    });
    const h = geometry.compact,
      font = h * .4;
    assert.ok(data.panel.width <= 36 * data.rootEm + .05 && data.panel.width <= data.viewport - 1);
    assert.equal(data.panel.extent, h / 2);
    assert.deepEqual(data.panel.border, ['1px', '1px', '1px', '1px']);
    assert.equal(parseFloat(data.formGap), 0);
    assert.equal(parseFloat(data.fieldGap), data.viewport > 768 ? h / 2 : 0);
    assert.equal(parseFloat(data.columnGap), 0);
    for (const [index, pair] of data.labels.entries()) {
      const i = pair.input,
        l = pair.span;
      assert.equal(i.height, h);
      assert.equal(i.font, font);
      assert.equal(i.extent, h / 2);
      assert.ok(Math.abs(i.padLeft - (h * .3 - 1)) < .05);
      assert.equal(l.height, h);
      assert.equal(l.font, font);
      if (data.viewport > 768) assert.ok(Math.abs((l.top + l.bottom - i.top - i.bottom) / 2) < .05);else assert.ok(Math.abs(i.top - l.bottom) < .05);
      if (index) {
        const previous = data.labels[index - 1];
        assert.ok(Math.abs(l.top - previous.input.bottom - (data.viewport > 768 ? h / 2 : 0)) < .05, name + ': desktop painted inputs get half-row spacing; phone labels supply their own row');
        assert.ok(i.top - previous.input.bottom >= h / 2 - .05, name + ': adjacent input contours remain separated');
      }
    }
    assert.equal(data.labels[0].input.readOnly, true);
    assert.deepEqual(data.labels.map(x => x.input.autocomplete), ['username', 'current-password', 'new-password', 'new-password']);
    assert.ok(Math.abs(data.save.height - h) < .05);
    assert.equal(data.save.font, font);
    assert.ok(data.document <= data.viewport);
    state.accountGeometry ??= [];
    state.accountGeometry.push({
      name,
      ...data
    });
    return data;
  }
  async function fileRows(name) {
    const data = await page.locator('.zxs-file-list-panel').evaluate(panel => {
      const scroll = panel.querySelector('.zxs-table-scroll'),
        table = scroll.querySelector('table');
      return {
        viewport: innerWidth,
        document: document.documentElement.scrollWidth,
        scroll: {
          client: scroll.clientWidth,
          width: scroll.scrollWidth
        },
        table: table.getBoundingClientRect().width,
        rows: [...table.rows].map(r => r.getBoundingClientRect().height),
        cells: [...table.rows].flatMap((row, rowIndex) => [...row.cells].map((cell, columnIndex) => {
          const style = getComputedStyle(cell),
            bounds = cell.getBoundingClientRect(),
            walker = document.createTreeWalker(cell, NodeFilter.SHOW_TEXT);
          let node,
            textStart = null;
          while ((node = walker.nextNode())) {
            if (node.parentElement.closest('svg')) continue;
            const start = node.textContent.search(/\S/u);
            if (start < 0) continue;
            const range = document.createRange();
            range.setStart(node, start);
            range.setEnd(node, start + String.fromCodePoint(node.textContent.codePointAt(start)).length);
            textStart = range.getBoundingClientRect().left - bounds.left;
            break;
          }
          const icon = cell.querySelector('.zxs-file-name > svg'),
            iconOffset = icon ? icon.getBoundingClientRect().width + (parseFloat(getComputedStyle(icon.parentElement).columnGap) || 0) : 0,
            contentStart = parseFloat(style.borderLeftWidth) + parseFloat(style.paddingLeft);
          return {
            row: rowIndex,
            column: columnIndex,
            tag: cell.tagName,
            textAlign: style.textAlign,
            contentStart,
            iconOffset,
            textStart,
            expectedTextStart: contentStart + iconOffset
          };
        })),
        lastBorders: [...(table.tBodies[0]?.rows[table.tBodies[0].rows.length - 1]?.cells || [])].map(c => getComputedStyle(c).borderBottomWidth)
      };
    });
    assert.ok(data.document <= data.viewport);
    assert.ok(data.rows.every(h => Math.abs(h - geometry.compact) < .02));
    assert.equal(data.cells.filter(cell => cell.tag === 'TH').length, 4);
    assert.ok(data.cells.some(cell => cell.tag === 'TD'));
    for (const cell of data.cells) {
      const label = `${name} row ${cell.row} column ${cell.column} ${cell.tag}`;
      assert.ok(['start', 'left'].includes(cell.textAlign), `${label} must align to the start, including browser-default th styles`);
      assert.notEqual(cell.textStart, null, `${label} must contain measurable text`);
      assert.ok(Math.abs(cell.textStart - cell.expectedTextStart) <= .5, `${label} first glyph must start at cell padding, after its optional file icon; actual ${cell.textStart}, expected ${cell.expectedTextStart}`);
    }
    if (data.table <= data.scroll.client + 1) assert.ok(data.scroll.width <= data.scroll.client + 1, 'fitting lists must not create horizontal scrollbars');
    assert.ok(data.lastBorders.every(x => x === '0px'));
    state.fileRows ??= [];
    state.fileRows.push({
      name,
      ...data
    });
  }
  async function loginKnownUser() {
    await page.getByLabel(t.user, {
      exact: true
    }).fill('admin');
    await page.getByLabel(t.password, {
      exact: true
    }).fill(password);
    await page.getByRole('button', {
      name: t.login,
      exact: true
    }).click();
    await page.locator('.zxs-disk-entry').first().waitFor({
      timeout: 60000
    });
    await Promise.allSettled(assetReads);
  }
  async function uiLogout() {
    const response = page.waitForResponse(r => new URL(r.url()).pathname === '/v1/auth/logout');
    await page.getByRole('button', {
      name: t.logout,
      exact: true
    }).click();
    const result = await response;
    assert.ok([200, 204, 401].includes(result.status()), 'current credentials should revoke the server session');
    await page.getByLabel(t.user, {
      exact: true
    }).waitFor();
    state.uiLogoutStatuses ??= [];
    state.uiLogoutStatuses.push(result.status());
    return result.status();
  }
  try {
    await page.goto(`${base}/?lang=${language}`);
    await page.getByLabel(t.user, {
      exact: true
    }).waitFor();
    await page.evaluate(() => document.fonts.ready);
    await check('login-established-size-and-g4-regression', async () => {
      state.originalLogin = await loginAppearance();
      await snapshot('login', {
        authPage: true
      });
    });
    await loginKnownUser();
    await check('disk-list-shared-roles-and-no-empty-shell-slots', async () => {
      await sharedRoles('disks');
      await flexGaps('disks');
      await fileRows('disks');
      await snapshot('disks');
    });
    await page.locator('.zxs-disk-entry[data-slot=P01]').click();
    await page.locator('.zxs-file-table tbody').getByRole('button', {
      name: seed.folder,
      exact: true
    }).click();
    await check('file-toolbar-list-and-single-source-row-geometry', async () => {
      await sharedRoles('files');
      await flexGaps('files');
      await blockGaps('files', ['.zxs-file-manager']);
      await fileRows('files');
      await ordinaryInput('ordinary-input-default');
      await snapshot('files');
      await shot('files');
    });
    await page.locator('.zxs-file-table tbody tr').filter({
      has: page.getByRole('button', {
        name: '代码',
        exact: true
      })
    }).getByRole('button', {
      name: t.details,
      exact: true
    }).click();
    await check('folder-details-shared-frames-rows-and-blocks', async () => {
      await sharedRoles('folder-details');
      await detailGeometry('folder-details');
      await flexGaps('folder-details');
      await snapshot('folder-details');
      await shot('folder-details');
    });
    await page.getByRole('button', {
      name: language === 'zh' ? '返回文件列表' : 'Back to files',
      exact: true
    }).click();
    await page.locator('.zxs-file-table tbody tr').filter({
      has: page.getByRole('button', {
        name: '说明.md',
        exact: true
      })
    }).getByRole('button', {
      name: t.details,
      exact: true
    }).click();
    await page.locator('.zxs-version-node').first().waitFor();
    await check('real-file-details-latest-first-real-parent-arrows', async () => {
      const h = await getReal(historyPath);
      state.realVersionMetadata = h.versions.map(v => ({
        version_id: v.version_id,
        source_version_ids: v.source_version_ids,
        created_at: v.created_at
      }));
      await sharedRoles('file-details-real');
      await detailGeometry('file-details-real');
      await assertReverse('real-newest-top', h.versions);
      await snapshot('file-details-real');
      await shot('file-details-real');
    });
    await check('multibranch-newest-leaves-top-upward-arrows-avoid-nodes', async () => {
      const h = await getReal(historyPath),
        versions = [version('oldest', [], 1), version('left', ['oldest'], 2, '旧备注跨行保留。'.repeat(8)), version('right', ['oldest'], 3), version('leaf', ['left'], 5), version('merge', ['left', 'right', 'oldest'], 6, 'latest', true), version('orphan', ['absent'], 4, 'separate')];
      state.treeFixture = {
        ...h,
        note: '简短备注 / Short note',
        versions
      };
      mockHistory = state.treeFixture;
      await refreshDetails();
      await assertReverse('mock-newest-top', versions);
      assert.equal(await nodeById('oldest').evaluate(e => e.parentElement.dataset.versionDepth), '2');
      assert.deepEqual(state.reverseTopology.at(-1).topLayer, ['merge', 'leaf', 'orphan']);
      await sharedRoles('file-details-mock');
      await detailGeometry('file-details-mock');
      await snapshot('file-details-mock');
      await shot('file-details-tree');
    });
    await check('inline-edit-limit-ime-and-cancel-no-business-post', async () => {
      const article = page.locator('.zxs-entry-content'),
        edit = article.getByRole('button', {
          name: language === 'zh' ? '编辑文件备注' : 'Edit file note',
          exact: true
        });
      await edit.click();
      const fileInput = article.locator('section > .zxs-inline-edit input.zxs-inline-input');
      await fileInput.fill('cancelled-ui-probe');
      await sharedRoles('file-note-edit');
      await fileInput.press('Escape');
      assert.equal(await article.locator('input.zxs-inline-input').count(), 0);
      const n = nodeById('left');
      await n.getByRole('button', {
        name: t.edit,
        exact: true
      }).click();
      const input = n.locator('input.zxs-inline-input');
      await input.fill('a'.repeat(129));
      await input.press('Enter');
      assert.equal(await input.getAttribute('aria-invalid'), 'true');
      await stableObserve('editing-upward-arrows', mockHistory.versions.map(v => v.version_id), ['oldest->left', 'oldest->right', 'left->leaf', 'left->merge', 'right->merge', 'oldest->merge']);
      await sharedRoles('version-note-edit');
      await input.evaluate(e => e.dispatchEvent(new KeyboardEvent('keydown', {
        key: 'Enter',
        isComposing: true,
        bubbles: true,
        cancelable: true
      })));
      assert.equal(await input.count(), 1, 'IME Enter must not submit');
      await input.press('Escape');
      await nodeById('merge').getByRole('button', {
        name: t.branch,
        exact: true
      }).click();
      const branchInput = page.locator('.zxs-version-actions input.zxs-branch-name');
      await branchInput.fill('cancelled-branch');
      await sharedRoles('version-branch-edit');
      await page.locator('.zxs-entry-navigation button').first().focus();
      await eventually(async () => (await branchInput.count()) === 0, 'blur cancels branch input');
      assert.equal(state.mockWrites.length, 0);
      assert.equal(state.unexpectedWrites.length, 0);
      mockHistory = null;
    });
    await menu(t.history).click();
    await check('real-history-list-shared-frames-and-wrapped-rows', async () => {
      await page.locator('.zxs-history-list-panel').waitFor();
      await eventually(async () => !(await page.locator('.zxs-history-page .zxs-loading').count()), 'history completes');
      await sharedRoles('history-real');
      await historyGeometry('history-real');
      await inspectSelect('history-select-default', '.zxs-history-toolbar .zxs-select-trigger');
      await snapshot('history-real');
      await page.screenshot({
        path: `${output}/history-real-${key}.png`,
        fullPage: false
      });
    });
    await check('history-long-rows-and-empty-without-ghost-frame', async () => {
      mockEvents = {
        events: [{
          timestamp: '2026-01-06T01:02:03Z',
          action: 'save',
          slot_id: 'P01',
          path: 'demo.txt',
          principal: 'admin'
        }, {
          timestamp: '2026-01-05T01:02:03Z',
          action: 'metadata',
          slot_id: 'P01',
          path: '长目录名称/'.repeat(8) + '文件.txt',
          principal: 'reader'
        }, {
          timestamp: '2026-01-04T01:02:03Z',
          action: 'branch',
          slot_id: 'P01',
          path: 'branch.txt',
          principal: ''
        }]
      };
      let response = page.waitForResponse(r => new URL(r.url()).pathname === '/v1/file-manager/history' && r.ok());
      await page.getByRole('button', {
        name: t.refresh,
        exact: true
      }).click();
      await response;
      await settle();
      await sharedRoles('history-mock');
      const data = await historyGeometry('history-mock');
      assert.equal(data.rows.length, 3);
      await snapshot('history-mock');
      await shot('history-mock');
      mockEvents = {
        events: []
      };
      response = page.waitForResponse(r => new URL(r.url()).pathname === '/v1/file-manager/history' && r.ok());
      await page.getByRole('button', {
        name: t.refresh,
        exact: true
      }).click();
      await response;
      await page.locator('.zxs-history-page .zxs-empty').waitFor();
      assert.equal(await page.locator('.zxs-history-list-panel').count(), 0);
      await sharedRoles('history-empty');
      await rowRecords('empty-history', ['.zxs-history-page .zxs-empty'], {
        padding: 10.8
      });
      await snapshot('history-empty');
      state.emptyHistoryWithoutGhostFrame = true;
      mockEvents = null;
    });
    await menu(t.status).click();
    await page.locator('.zxs-metric-card').nth(8).waitFor();
    await check('monitor-surface-clearance-card-grid-and-unframed-footer', async () => {
      await sharedRoles('status');
      await rowRecords('sampled-at', ['.zxs-metric-timestamp'], {
        padding: 10.8,
        exact: width >= 800
      });
      await rowRecords('status-card-rows', ['.zxs-metric-heading', '.zxs-metric-subtitle'], {
        padding: 0,
        exact: true
      });
      await blockGaps('status', ['.zxs-monitor-page']);
      await statusLayout('current-status');
      await flexGaps('status');
      const gaps = await page.locator('.zxs-metric-grid').evaluate(e => ({
        row: parseFloat(getComputedStyle(e).rowGap),
        column: parseFloat(getComputedStyle(e).columnGap)
      }));
      assert.deepEqual(gaps, {
        row: 18,
        column: 18
      });
      state.statusGaps = gaps;
      await snapshot('status');
      await shot('status');
    });
    await page.getByRole('button', {
      name: t.account,
      exact: true
    }).click();
    await page.locator('.zxs-account-page').waitFor();
    await check('account-compact-width-labels-inputs-and-mobile-groups', async () => {
      await sharedRoles('account');
      await accountGeometry('account');
      await blockGaps('account-outer', ['.zxs-shell-main > .zxs-content-stack']);
      await snapshot('account');
      await shot('account');
    });
    await check('account-local-errors-and-intercepted-dummy-password', async () => {
      const account = page.locator('.zxs-account-page'),
        form = account.locator('form');
      await account.getByRole('button', {
        name: t.save,
        exact: true
      }).click();
      await account.locator('.zxs-error').waitFor();
      assert.equal(state.mockWrites.length, 0);
      await accountGeometry('local-empty-error');
      await account.getByLabel(t.current, {
        exact: true
      }).fill('synthetic-current-unused');
      await account.getByLabel(t.new, {
        exact: true
      }).fill('synthetic-new-unused');
      await account.getByLabel(t.confirm, {
        exact: true
      }).fill('different-unused');
      await account.getByRole('button', {
        name: t.save,
        exact: true
      }).click();
      await account.locator('.zxs-error').waitFor();
      assert.equal(state.mockWrites.length, 0);
      await accountGeometry('local-mismatch-error');
      await account.getByLabel(t.confirm, {
        exact: true
      }).fill('synthetic-new-unused');
      const plan = {
        captured: false
      };
      passwordPlan = plan;
      await account.getByRole('button', {
        name: t.save,
        exact: true
      }).click();
      await eventually(() => plan.captured, 'mock password request captured');
      assert.equal(await form.getAttribute('aria-busy'), 'true');
      assert.equal(await account.getByRole('button', {
        name: t.saving,
        exact: true
      }).isDisabled(), true);
      const inputs = await account.locator('input[type=password]').evaluateAll(es => es.map(e => ({
        readOnly: e.readOnly,
        valueLength: e.value.length
      })));
      assert.ok(inputs.every(i => i.readOnly && i.valueLength === 0));
      await accountGeometry('pending');
      await snapshot('account-pending');
      plan.release();
      await account.getByText(/ZXS_LAYOUT_MOCK/).waitFor();
      await eventually(async () => (await form.getAttribute('aria-busy')) === 'false', 'pending clears after mock failure');
      assert.ok((await account.locator('input[type=password]').evaluateAll(es => es.map(e => e.readOnly))).every(v => !v));
      await sharedRoles('account-error');
      await accountGeometry('remote-mock-error');
      await snapshot('account-error');
      await shot('account-error');
    });
    await check('live-shared-token-update-propagates-all-pages-and-g4', async () => {
      await Promise.allSettled(assetReads);
      tokenMode = true;
      geometry = {
        compact: 40,
        comfortable: 50,
        ratio: .4
      };
      await page.reload();
      await page.locator('.zxs-disk-entry').first().waitFor();
      await sharedRoles('token-disks');
      await fileRows('token-disks');
      await snapshot('token-disks');
      await page.locator('.zxs-disk-entry[data-slot=P01]').click();
      await page.locator('.zxs-file-table tbody').getByRole('button', {
        name: seed.folder,
        exact: true
      }).click();
      await sharedRoles('token-files');
      await flexGaps('token-files');
      await fileRows('token-files');
      await ordinaryInput('ordinary-input-token');
      await page.locator('.zxs-file-table tbody tr').filter({
        has: page.getByRole('button', {
          name: '说明.md',
          exact: true
        })
      }).getByRole('button', {
        name: t.details,
        exact: true
      }).click();
      await page.locator('.zxs-version-node').first().waitFor();
      mockHistory = state.treeFixture;
      await refreshDetails();
      await sharedRoles('token-details');
      await assertReverse('token-upward-tree', mockHistory.versions);
      await snapshot('token-details');
      await shot('token-details');
      mockHistory = null;
      await menu(t.history).click();
      await page.locator('.zxs-history-list-panel').waitFor();
      await sharedRoles('token-history');
      await flexGaps('token-history');
      await inspectSelect('history-select-token', '.zxs-history-toolbar .zxs-select-trigger');
      await snapshot('token-history');
      await menu(t.status).click();
      await page.locator('.zxs-metric-card').nth(8).waitFor();
      await sharedRoles('token-status');
      await flexGaps('token-status');
      const tokenMetricGap = await page.locator('.zxs-metric-grid').evaluate(e => [getComputedStyle(e).rowGap, getComputedStyle(e).columnGap]);
      assert.deepEqual(tokenMetricGap, ['20px', '20px']);
      await snapshot('token-status');
      await shot('token-status');
      await page.getByRole('button', {
        name: t.account,
        exact: true
      }).click();
      await page.locator('.zxs-account-page').waitFor();
      await sharedRoles('token-account');
      await accountGeometry('token-account');
      await snapshot('token-account');
      await shot('token-account');
      state.tokenPropagation = {
        compact: 40,
        comfortable: 50,
        font: 16,
        defaultStackGap: 0,
        defaultFlowGap: 0,
        surfaceGap: 20,
        metricAndTreeGap: 20,
        frameExtent: 20,
        ordinaryExtent: 25,
        allPages: true
      };
    });
    await check('old-401-after-new-login-cannot-clear-new-credentials', async () => {
      await menu(t.history).click();
      await page.locator('.zxs-history-list-panel').waitFor();
      const plan = {
        path: '/v1/file-manager/history',
        captured: false
      };
      holdGetPlan = plan;
      await page.getByRole('button', {
        name: t.refresh,
        exact: true
      }).click();
      await eventually(() => plan.captured, 'old history read is held');
      await uiLogout();
      const after = await loginAppearance();
      assert.deepEqual(after, state.originalLogin, 'business token changes must leave the actual login unchanged');
      await snapshot('login-after-token', {
        authPage: true
      });
      await loginKnownUser();
      const response = page.waitForResponse(r => new URL(r.url()).pathname === plan.path && r.status() === 401);
      plan.release();
      await (await response).finished();
      await settle();
      assert.equal(await page.locator('.zxs-app-shell').count(), 1);
      assert.equal(await page.locator('.zxs-auth-shell').count(), 0);
      await sharedRoles('late-401-new-session');
      await snapshot('late-401-new-session');
      // This real logout proves the new CSRF/TOKEN survived; a stale clear would
      // keep the cookie but drop CSRF and make the request fail with 403.
      await uiLogout();
      state.loggedOut = true;
      state.staleUnauthorizedProtected = true;
      await loginAppearance();
    });
    await check('candidate-assets-dynamic-css-inventory-csp-and-write-guard', async () => {
      await Promise.allSettled(assetReads);
      assert.deepEqual(state.pageErrors, []);
      state.unexpectedConsoleErrors = state.consoleErrors.filter(message => !/Failed to load resource:.*status(?: of)? (?:400|401)\b/.test(message));
      assert.deepEqual(state.unexpectedConsoleErrors, []);
      assert.deepEqual(state.unexpectedWrites, []);
      assert.deepEqual(await page.evaluate(() => window.__zxsCspViolations), []);
      assert.deepEqual(state.assetReadErrors || [], []);
      const baselinePaths = new Set(state.assets.filter(a => !a.testTokenOverride).map(a => a.path));
      assert.deepEqual([...baselinePaths].sort(), expectedResources, 'every candidate CSS file and WASM must have actually been requested');
      assert.ok(state.assets.every(a => a.status === 200 && a.same));
      for (const asset of state.assets) {
        assert.ok(asset.mime?.includes(asset.path.endsWith('.wasm') ? 'application/wasm' : 'text/css'));
        if (!asset.testTokenOverride) assert.ok(asset.etag, 'candidate static responses expose an ETag');
      }
      assert.ok(state.wasm.length && state.wasm.every(w => w.status === 200 && w.mime?.includes('application/wasm')));
      assert.equal(state.mockWrites.length, 1);
      assert.ok(state.staleUnauthorizedProtected);
      state.expectedResources = expectedResources;
    });
  } catch (e) {
    state.failure = clean(e);
    try {
      await shot('failure');
    } catch {}
  } finally {
    for (const release of held) release();
    await Promise.allSettled(assetReads);
    if (loginInFlight) try {
      await page.waitForResponse(r => new URL(r.url()).pathname === '/v1/auth/login', {
        timeout: 60000
      });
    } catch {
      state.loginSettlementUnconfirmed = true;
    }
    if (!state.loggedOut) try {
      if (await page.getByRole('button', {
        name: t.logout,
        exact: true
      }).count()) {
        await uiLogout();
        state.loggedOut = true;
      }
    } catch (e) {
      state.uiLogoutFailure = clean(e);
    }
    if (!state.loggedOut && auth?.token) try {
      const r = await context.request.post(base + '/v1/auth/logout', {
        headers: {
          Authorization: `Bearer ${auth.token}`
        },
        data: {}
      });
      state.cleanupRevoked = [200, 204, 401].includes(r.status());
    } catch {}
    await context.close();
  }
  state.passed = !state.failure && state.loggedOut;
  delete state.treeFixture;
  scenes.push(state);
  await writeFile(`${output}/result-${key}.json`, JSON.stringify(state, null, 2) + '\n');
  console.log(JSON.stringify({
    language,
    theme,
    width,
    passed: state.passed,
    checks: state.checks.length,
    mockWrites: state.mockWrites.length,
    loggedOut: state.loggedOut,
    ...(state.failure ? {
      failure: state.failure
    } : {})
  }));
}
try {
  for (const [language, theme, width] of process.env.ZXS_MAINT_SCENES ? JSON.parse(process.env.ZXS_MAINT_SCENES) : [['zh', 'light', 1440], ['zh', 'dark', 390], ['en', 'light', 800], ['en', 'dark', 320]]) await scene(language, theme, width);
} finally {
  await browser.close();
}
const result = {
  passed: scenes.every(s => s.passed),
  allBusinessWritesIntercepted: true,
  normalViewportsOnly: true,
  scenes
};
await writeFile(`${output}/result.json`, JSON.stringify(result, null, 2) + '\n');
const summary = {
  passed: result.passed,
  scenes: scenes.length,
  checks: scenes.reduce((sum, s) => sum + s.checks.length, 0),
  passedChecks: scenes.reduce((sum, s) => sum + s.checks.filter(c => c.passed).length, 0),
  normalViewportsOnly: true,
  allBusinessWritesIntercepted: true,
  results: scenes.map(s => ({
    language: s.language,
    theme: s.theme,
    width: s.width,
    passed: s.passed,
    loggedOut: s.loggedOut,
    checks: s.checks.length,
    tokenPropagation: s.tokenPropagation,
    staleUnauthorizedProtected: s.staleUnauthorizedProtected,
    ...(s.failure ? {
      failure: s.failure
    } : {})
  }))
};
await writeFile(`${output}/summary.json`, JSON.stringify(summary, null, 2) + '\n');
console.log(JSON.stringify(summary));
if (!result.passed) process.exitCode = 1;
