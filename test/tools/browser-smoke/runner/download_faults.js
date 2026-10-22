// Test-only controlled fault streams and actual OPFS writes. Native OS dialog unverified.

    window.__downloadOriginalFetch = window.fetch;
    window.__downloadOriginalPicker = window.showSaveFilePicker;
    window.__downloadOriginalEstimate = navigator.storage.estimate;
    window.__downloadOriginalDirectory = navigator.storage.getDirectory;
    window.__downloadProbe = {};
    window.showSaveFilePicker = function () {
      const p = window.__downloadProbe;
      p.activation = navigator.userActivation.isActive;
      p.picked++;
      if (p.mode === 'picker-cancel') return Promise.reject(new DOMException('cancel', 'AbortError'));
      return (async () => {
        const root = await navigator.storage.getDirectory();
        const name = `zxs-qa-picker-${crypto.randomUUID()}`;
        const handle = await root.getFileHandle(name, {create: true});
        p.cleanup = () => root.removeEntry(name);
        return {createWritable: async () => {
          const writable = await handle.createWritable();
          return {
            write: async bytes => {
              p.writes++;
              p.maxChunk = Math.max(p.maxChunk, bytes.byteLength);
              p.inflight++;
              p.maxInflight = Math.max(p.maxInflight, p.inflight);
              try {
                if (p.mode === 'quota') throw new DOMException('injected quota exhaustion', 'QuotaExceededError');
                if (p.mode === 'cancel' || p.mode === 'logout') {
                  await new Promise((resolve, reject) => { p.releaseWrite = reject; });
                }
                await writable.write(bytes);
                if (p.mode === 'uncooperative-write') {
                  p.writeWaiting = true;
                  await new Promise(resolve => { p.releaseWrite = () => { p.writeWaiting=false; resolve(); }; });
                }
                p.bytes += bytes.byteLength;
              } finally { p.inflight--; }
            },
            close: async () => {
              p.closeCalls++;
              if (p.mode === 'close-failure') throw new DOMException('injected close failure', 'QuotaExceededError');
              await writable.close();
              const saved = await handle.getFile();
              p.savedSize = saved.size;
              if (saved.size <= 3) p.savedText = await saved.text();
              p.committed = true;
            },
            abort: async () => {
              p.abortCalls = (p.abortCalls || 0) + 1;
              if (p.mode === 'uncooperative-write') {
                p.abortWaiting = true;
                await new Promise(resolve => { p.releaseAbort = () => { p.abortWaiting=false; resolve(); }; });
              } else {
                p.releaseWrite?.(new DOMException('cancel', 'AbortError'));
                p.releaseWrite = null;
              }
              await writable.abort();
              p.aborted++;
            }
          };
        }};
      })();
    };
    window.fetch = async function(input, init) {
      const request = input instanceof Request ? input : new Request(input, init);
      if (new URL(request.url).pathname === '/api/capabilities') window.__downloadProbe.capabilities=(window.__downloadProbe.capabilities||0)+1;
      if (new URL(request.url).pathname === '/api/capabilities' && window.__downloadProbe.mode?.endsWith('-timeout')) {
        const response = await window.__downloadOriginalFetch.call(this, input, init);
        if (!response.ok) return response;
        const value = await response.json();
        value.transfer_idle_timeout_ms = window.__downloadProbe.mode === 'idle-timeout' ? '500' : '1000';
        value.transfer_total_timeout_ms = '1500';
        return new Response(JSON.stringify(value), {status:response.status, headers:response.headers});
      }
      if (!/\/api\/versions\/[^/]+\/content$/.test(new URL(request.url).pathname)) {
        return window.__downloadOriginalFetch.call(this, input, init);
      }
      const p = window.__downloadProbe;
      p.fetched++;
      request.signal.addEventListener('abort', () => { p.fetchAborted = true; });
      const bytes = new TextEncoder().encode(p.mode.includes('corruption') ? 'abd' : 'abc');
      let sent = false;
      let largeChunks = 0;
      let timer;
      return new Response(new ReadableStream({
        start(controller) {
          request.signal.addEventListener('abort', () => {
            clearTimeout(timer);
            try { controller.error(new DOMException('aborted', 'AbortError')); } catch {}
          });
        },
        async pull(controller) {
          if (p.mode === 'idle-timeout') return new Promise(() => {});
          if (p.mode === 'total-timeout') {
            await new Promise(resolve => { timer = setTimeout(resolve, 100); });
            if (!request.signal.aborted) controller.enqueue(new Uint8Array(1));
            return;
          }
          if (['valid-large', 'fallback-large'].includes(p.mode)) {
            if (largeChunks++ < 65) controller.enqueue(new Uint8Array(1024 * 1024));
            else controller.close();
            return;
          }
          if (sent) controller.close();
          else { sent = true; controller.enqueue(bytes); }
        },
        cancel() { p.readerCancelled = true; clearTimeout(timer); }
      }), {status: 200, headers: {
        'Content-Length': String(p.mode.endsWith('-timeout') ? 100 : p.mode === 'overrun' ? 2 : p.mode === 'truncated' ? 4 : ['fallback-large', 'valid-large'].includes(p.mode) ? 68157440 : 3),
        'ETag': ['valid-large', 'fallback-large'].includes(p.mode) ? '"c7d5e9a4a234b48a92a436fcb54145065325e8a17b0d2f5aced359fa505a95f7"' : '"6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"'
      }});
    };
return true;
