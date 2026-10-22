// Import into a real-browser acceptance runner after login and opening file details.
// Exercises shipped WASM through its real click handler. Only the browser disk sink
// is a controlled picker returning actual OPFS handles; only download responses and
// injected errors are synthetic. Native OS picker and physical disk durability are unverified.
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';

export async function verifyDownloads(page) {
  const download = page.getByRole('button', {name: /^(Download|下载)$/});
  const cancel = page.getByRole('button', {name: /^(Cancel download|取消下载)$/});
  const cases = [];
  const fixture = await readFile(new URL('../../tools/browser-smoke/runner/download_faults.js', import.meta.url), 'utf8');
  await page.evaluate(`(() => {${fixture}})()`);
  try {
    for (const mode of ['valid', 'valid-large', 'corruption', 'truncated', 'overrun', 'quota', 'close-failure', 'picker-cancel', 'cancel', 'idle-timeout', 'total-timeout']) {
      await page.evaluate(mode => {
        window.__downloadProbe = {mode, picked: 0, writes: 0, maxChunk: 0, inflight: 0, maxInflight: 0, bytes: 0, closeCalls: 0, committed: false, aborted: 0, fetched: 0};
        if (mode === 'fallback-large') window.showSaveFilePicker = undefined;
      }, mode);
      await download.click();
      if (mode === 'cancel') {
        await page.waitForFunction(() => window.__downloadProbe.writes === 1);
        await cancel.click();
      }
      await page.waitForFunction(() => !Array.from(document.querySelectorAll('button')).some(b => /^(Cancel download|取消下载)$/.test(b.textContent.trim())));
      const result = await page.evaluate(() => {
        const {releaseWrite, cleanup, ...state} = window.__downloadProbe;
        return state;
      });
      assert.ok(result.maxInflight <= 1, `${mode}: backpressure`);
      assert.ok(result.maxChunk <= 256 * 1024, `${mode}: bounded chunks`);
      assert.equal(result.committed, ['valid', 'valid-large'].includes(mode), `${mode}: commit gate`);
      assert.equal(result.closeCalls, ['valid', 'valid-large', 'close-failure'].includes(mode) ? 1 : 0, `${mode}: no premature close`);
      if (!['picker-cancel', 'fallback-large'].includes(mode)) assert.equal(result.activation, true, `${mode}: click activation`);
      if (mode === 'picker-cancel') assert.equal(result.fetched, 0);
      if (mode === 'valid-large') { assert.equal(result.bytes, 65 * 1024 * 1024); assert.equal(result.savedSize, result.bytes); }
      if (mode === 'valid') assert.equal(result.savedText, 'abc');
      if (mode === 'cancel' || mode.endsWith('-timeout')) assert.ok(result.fetchAborted && result.aborted, `${mode}: abort network and actual writable`);
      cases.push(result);
      await page.evaluate(async () => { await window.__downloadProbe.cleanup?.(); });
    }
    await page.evaluate(() => {
      window.showSaveFilePicker=undefined;
      navigator.storage.getDirectory=undefined;
      window.__downloadProbe={mode:'unsupported',fetched:0,capabilities:0};
    });
    await download.click();
    await page.waitForFunction(() => document.body.textContent.includes('DOWNLOAD_STORAGE_UNSUPPORTED'));
    assert.deepEqual(await page.evaluate(() => ({fetched:window.__downloadProbe.fetched,capabilities:window.__downloadProbe.capabilities})), {fetched:0,capabilities:0});
    await page.evaluate(() => { navigator.storage.getDirectory=window.__downloadOriginalDirectory; });
    cases.push({mode:'unsupported',rejectedBeforeFetch:true});
    for (const mode of ['fallback-corruption', 'fallback-quota']) {
      await page.evaluate(mode => {
        window.showSaveFilePicker = undefined;
        window.__downloadProbe = {mode, fetched:0};
        if (mode === 'fallback-quota') navigator.storage.estimate = async () => ({usage:0,quota:1});
      }, mode);
      await download.click();
      await page.waitForFunction(() => !Array.from(document.querySelectorAll('button')).some(b => /^(Cancel download|取消下载)$/.test(b.textContent.trim())));
      assert.equal(await page.getByRole('button', {name:/^(Save to device|保存到设备)$/}).count(), 0, `${mode}: no export after failure`);
      assert.equal(await page.evaluate(async () => {
        const root = await navigator.storage.getDirectory();
        const parent = await root.getDirectoryHandle('zxs-verified-downloads');
        let count=0; for await (const entry of parent.entries()) count++;
        return count;
      }), 0, `${mode}: failed spool removed`);
      await page.evaluate(() => { navigator.storage.estimate=window.__downloadOriginalEstimate; });
      cases.push({mode,actualOPFS:true});
    }
    // Production OPFS fallback, not the controlled picker. Browser export is
    // initiated only by a fresh click and backing bytes must remain until cleanup.
    for (const exportFirst of [false, true]) {
      await page.evaluate(() => {
        window.showSaveFilePicker = undefined;
        window.__downloadProbe = {mode:'fallback-large', picked:0, fetched:0};
      });
      await download.click();
      const save = page.getByRole('button', {name: /^(Save to device|保存到设备)$/});
      await save.waitFor({state:'visible'});
      const retained = await page.evaluate(async () => {
        const root = await navigator.storage.getDirectory();
        const directory = await root.getDirectoryHandle('zxs-verified-downloads');
        const names = [];
        for await (const [name] of directory.entries()) names.push(name);
        return names;
      });
      assert.equal(retained.length, 1, 'exactly one retained verified export');
      // A second real button click must not start a competing content request.
      await download.evaluate(button => button.click());
      assert.equal(await page.evaluate(() => window.__downloadProbe.fetched), 1, 'single active/retained job');
      if (exportFirst) {
        const exported = page.waitForEvent('download');
        await save.click();
        const result = await exported;
        assert.equal(await result.failure(), null, 'browser export succeeds');
        assert.equal(await page.evaluate(async name => {
          const root = await navigator.storage.getDirectory();
          const parent = await root.getDirectoryHandle('zxs-verified-downloads');
          const dir = await parent.getDirectoryHandle(name);
          return (await (await dir.getFileHandle('content')).getFile()).size;
        }, retained[0]), 65 * 1024 * 1024, 'export backing retained after anchor click');
        await page.getByRole('button', {name:/^(Clean up after saving|保存完成后清理临时副本)$/}).click();
      } else {
        await page.getByRole('button', {name:/^(Cancel and clean up|取消并清理)$/}).click();
      }
      await page.waitForFunction(async name => {
        const root = await navigator.storage.getDirectory();
        const parent = await root.getDirectoryHandle('zxs-verified-downloads');
        try { await parent.getDirectoryHandle(name); return false; }
        catch(error) { if(error.name==='NotFoundError') return true; throw error; }
      }, retained[0]);
      cases.push({mode:exportFirst?'opfs-export-retained-cleanup':'opfs-cancel-cleanup', actualOPFS:true});
    }
  } finally {
    await page.evaluate(async () => {
      await window.__downloadProbe.cleanup?.().catch(() => {});
      window.fetch = window.__downloadOriginalFetch;
      window.showSaveFilePicker = window.__downloadOriginalPicker;
      navigator.storage.estimate=window.__downloadOriginalEstimate;
      navigator.storage.getDirectory=window.__downloadOriginalDirectory;
      delete window.__downloadOriginalDirectory;
      delete window.__downloadOriginalEstimate;
      delete window.__downloadOriginalFetch;
      delete window.__downloadOriginalPicker;
    });
  }
  return cases;
}
