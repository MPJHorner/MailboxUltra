// MailBox Ultra — embedded web UI. Vanilla JS, no build step. The whole file
// is bundled into the binary via rust-embed and served by the UI router.
//
// XSS posture: every interpolated string goes through escapeHtml() before
// being assigned to innerHTML. The captured email HTML is rendered inside a
// sandboxed iframe (no allow-scripts) using a Blob URL so embedded JS in the
// captured message cannot execute or reach the parent document.

(() => {
  'use strict';

  const $ = (sel) => document.querySelector(sel);
  const $$ = (sel) => Array.from(document.querySelectorAll(sel));

  const state = {
    messages: [],
    selectedId: null,
    paused: false,
    filter: '',
    activeTab: 'html',
    previewDevice: 'desktop',
    relay: { enabled: false, url: null },
  };

  const fmtBytes = (n) => {
    if (n < 1024) return n + ' B';
    if (n < 1024 * 1024) return (n / 1024).toFixed(1) + ' KiB';
    return (n / (1024 * 1024)).toFixed(2) + ' MiB';
  };
  const fmtTime = (iso) => {
    try {
      const d = new Date(iso);
      const pad = (x) => String(x).padStart(2, '0');
      return pad(d.getHours()) + ':' + pad(d.getMinutes()) + ':' + pad(d.getSeconds());
    } catch (e) { return ''; }
  };
  const fmtDate = (iso) => {
    try { return new Date(iso).toLocaleString(); } catch (e) { return ''; }
  };
  const escapeHtml = (s) =>
    String(s == null ? '' : s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
  const matchFilter = (m, q) => {
    if (!q) return true;
    const needle = q.toLowerCase();
    return (
      (m.subject || '').toLowerCase().includes(needle) ||
      (m.from && (m.from.address || '').toLowerCase().includes(needle)) ||
      m.envelope_from.toLowerCase().includes(needle) ||
      (m.envelope_to || []).some((t) => t.toLowerCase().includes(needle)) ||
      (m.to || []).some((t) => (t.address || '').toLowerCase().includes(needle))
    );
  };
  const toast = (msg, kind) => {
    const t = $('#toast');
    t.textContent = msg;
    t.className = 'toast show ' + (kind || '');
    clearTimeout(toast._h);
    toast._h = setTimeout(() => { t.className = 'toast'; }, 2000);
  };
  const renderFromTo = (m) => {
    const from = (m.from && m.from.address) || m.envelope_from || '(unknown)';
    const to = (m.to[0] && m.to[0].address) || m.envelope_to[0] || '(unknown)';
    const more = (m.envelope_to.length > 1) ? ' +' + (m.envelope_to.length - 1) : '';
    return { from: from, to: to + more };
  };

  const THEME_KEY = 'mbu-theme';
  const applyTheme = (t) => { document.documentElement.dataset.theme = t; };
  const initTheme = () => {
    const saved = localStorage.getItem(THEME_KEY);
    if (saved === 'light' || saved === 'dark') applyTheme(saved);
  };
  const toggleTheme = () => {
    const next = document.documentElement.dataset.theme === 'dark' ? 'light' : 'dark';
    applyTheme(next);
    localStorage.setItem(THEME_KEY, next);
  };

  const api = {
    list: () => fetch('/api/messages?limit=10000').then((r) => r.json()),
    one: (id) => fetch('/api/messages/' + id).then((r) => r.ok ? r.json() : null),
    raw: (id) => fetch('/api/messages/' + id + '/raw').then((r) => r.text()),
    delete: (id) => fetch('/api/messages/' + id, { method: 'DELETE' }),
    clear: () => fetch('/api/messages', { method: 'DELETE' }),
    health: () => fetch('/api/health').then((r) => r.json()),
    relay: {
      get: () => fetch('/api/relay').then((r) => r.json()),
      put: (url, insecure) => fetch('/api/relay', {
        method: 'PUT',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ url: url, insecure: insecure }),
      }),
      del: () => fetch('/api/relay', { method: 'DELETE' }),
    },
    release: (id, url, insecure) => fetch('/api/messages/' + id + '/release', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ smtp_url: url, insecure: insecure }),
    }),
  };

  const renderList = () => {
    const list = $('#message-list');
    const empty = $('#empty-state');
    const visible = state.messages.filter((m) => matchFilter(m, state.filter));
    const html = visible.map((m) => {
      const ft = renderFromTo(m);
      const subj = m.subject || '(no subject)';
      const att = (m.attachments && m.attachments.length > 0)
        ? '<span class="msg-att" title="' + m.attachments.length + ' attachment(s)">📎' + m.attachments.length + '</span>'
        : '';
      return '<li class="message-item ' + (m.id === state.selectedId ? 'active' : '') + '" data-id="' + escapeHtml(m.id) + '" tabindex="0">' +
        '<span class="msg-from">' + escapeHtml(ft.from) + '</span>' +
        '<span class="msg-time">' + escapeHtml(fmtTime(m.received_at)) + '</span>' +
        '<span class="msg-subject">' + escapeHtml(subj) + ' ' + att + '</span>' +
        '<span class="msg-meta"><span class="msg-to">→ ' + escapeHtml(ft.to) + '</span><span>' + escapeHtml(fmtBytes(m.size)) + '</span></span>' +
        '</li>';
    }).join('');
    list.innerHTML = html;
    empty.classList.toggle('hidden', state.messages.length > 0);
    $('#count').textContent = state.messages.length;
  };

  const renderDetail = () => {
    const m = state.messages.find((x) => x.id === state.selectedId);
    const empty = $('#detail-empty');
    const detail = $('#detail');
    if (!m) {
      empty.hidden = false;
      detail.hidden = true;
      return;
    }
    empty.hidden = true;
    detail.hidden = false;

    const ft = renderFromTo(m);
    $('#d-from').textContent = ft.from;
    $('#d-to').textContent = ft.to;
    $('#d-time').textContent = fmtDate(m.received_at);
    $('#d-size').textContent = fmtBytes(m.size);
    $('#d-subject').textContent = m.subject || '(no subject)';
    $('#d-auth').hidden = !m.authenticated;

    $('#t-html-meta').textContent = m.html ? '' : '—';
    $('#t-text-meta').textContent = m.text ? '' : '—';
    $('#t-headers-meta').textContent = m.headers.length;
    $('#t-att-meta').textContent = m.attachments.length || '';

    // HTML pane: device-size toolbar + sandboxed iframe served from a Blob.
    const paneHtml = $('#pane-html');
    paneHtml.replaceChildren();
    if (m.html) {
      const devices = [
        { id: 'desktop', label: 'Desktop', dim: 'full',
          icon: 'M2 2h12a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1H9v1h2v1H5v-1h2v-1H2a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1Zm0 1v8h12V3H2Z' },
        { id: 'tablet',  label: 'iPad',    dim: '820',
          icon: 'M4 1h8a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V3a2 2 0 0 1 2-2Zm0 1a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1V3a1 1 0 0 0-1-1H4Zm3 11h2v1H7v-1Z' },
        { id: 'mobile',  label: 'Mobile',  dim: '390',
          icon: 'M5 1h6a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V3a2 2 0 0 1 2-2Zm0 1a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1V3a1 1 0 0 0-1-1H5Zm2 11h2v1H7v-1Z' },
      ];
      const bar = document.createElement('div');
      bar.className = 'device-bar';
      const SVG = 'http://www.w3.org/2000/svg';
      devices.forEach((d) => {
        const btn = document.createElement('button');
        btn.type = 'button';
        btn.className = 'device-btn' + (d.id === state.previewDevice ? ' active' : '');
        btn.dataset.device = d.id;
        const svg = document.createElementNS(SVG, 'svg');
        svg.setAttribute('viewBox', '0 0 16 16');
        svg.setAttribute('width', '13');
        svg.setAttribute('height', '13');
        svg.setAttribute('aria-hidden', 'true');
        const path = document.createElementNS(SVG, 'path');
        path.setAttribute('fill', 'currentColor');
        path.setAttribute('d', d.icon);
        svg.appendChild(path);
        const lbl = document.createElement('span');
        lbl.textContent = d.label;
        const dim = document.createElement('span');
        dim.className = 'device-dim';
        dim.textContent = d.dim;
        btn.appendChild(svg);
        btn.appendChild(lbl);
        btn.appendChild(dim);
        bar.appendChild(btn);
      });
      const stage = document.createElement('div');
      stage.className = 'iframe-stage device-' + state.previewDevice;
      const ifr = document.createElement('iframe');
      ifr.setAttribute('sandbox', 'allow-popups');
      ifr.setAttribute('referrerpolicy', 'no-referrer');
      const blob = new Blob([m.html], { type: 'text/html' });
      ifr.src = URL.createObjectURL(blob);
      stage.appendChild(ifr);
      paneHtml.appendChild(bar);
      paneHtml.appendChild(stage);
      bar.addEventListener('click', (e) => {
        const btn = e.target.closest('.device-btn');
        if (!btn) return;
        state.previewDevice = btn.dataset.device;
        bar.querySelectorAll('.device-btn').forEach((b) => b.classList.toggle('active', b.dataset.device === state.previewDevice));
        stage.className = 'iframe-stage device-' + state.previewDevice;
      });
    } else {
      const div = document.createElement('div');
      div.className = 'pane-empty';
      div.textContent = 'No HTML body in this message.';
      paneHtml.appendChild(div);
    }

    // Text pane: textContent only, no HTML rendering.
    const paneText = $('#pane-text');
    paneText.replaceChildren();
    if (m.text) {
      const pre = document.createElement('pre');
      pre.textContent = m.text;
      paneText.appendChild(pre);
    } else {
      const div = document.createElement('div');
      div.className = 'pane-empty';
      div.textContent = 'No plain text body in this message.';
      paneText.appendChild(div);
    }

    // Headers pane.
    const paneHeaders = $('#pane-headers');
    const tbl = document.createElement('table');
    tbl.className = 'headers-table';
    const tbody = document.createElement('tbody');
    m.headers.forEach((kv) => {
      const tr = document.createElement('tr');
      const tdK = document.createElement('td'); tdK.className = 'k'; tdK.textContent = kv[0];
      const tdV = document.createElement('td'); tdV.className = 'v'; tdV.textContent = kv[1];
      tr.appendChild(tdK); tr.appendChild(tdV);
      tbody.appendChild(tr);
    });
    tbl.appendChild(tbody);
    paneHeaders.replaceChildren(tbl);

    // Attachments pane.
    const paneAtt = $('#pane-attachments');
    paneAtt.replaceChildren();
    if (m.attachments.length === 0) {
      const div = document.createElement('div');
      div.className = 'pane-empty';
      div.textContent = 'No attachments.';
      paneAtt.appendChild(div);
    } else {
      const wrap = document.createElement('div');
      wrap.className = 'attachments-list';
      m.attachments.forEach((a, i) => {
        const row = document.createElement('div');
        row.className = 'attachment';
        const left = document.createElement('div');
        const name = document.createElement('div');
        name.className = 'att-name';
        name.textContent = a.filename || ('attachment-' + (i + 1));
        const meta = document.createElement('div');
        meta.className = 'att-meta';
        meta.textContent = a.content_type + ' · ' + fmtBytes(a.size);
        left.appendChild(name); left.appendChild(meta);
        const link = document.createElement('a');
        link.href = '/api/messages/' + m.id + '/attachments/' + i;
        link.setAttribute('download', '');
        link.textContent = 'Download';
        row.appendChild(left); row.appendChild(link);
        wrap.appendChild(row);
      });
      paneAtt.appendChild(wrap);
    }

    // Source pane.
    const paneSource = $('#pane-source');
    const srcPre = document.createElement('pre');
    srcPre.id = 'source-pre';
    srcPre.textContent = 'loading…';
    paneSource.replaceChildren(srcPre);
    api.raw(m.id).then((txt) => { srcPre.textContent = txt; });

    // Release pane.
    const paneRelease = $('#pane-release');
    const form = document.createElement('form');
    form.className = 'release-form';
    form.id = 'release-form';
    form.innerHTML =
      '<p>Forward this captured message to a real SMTP server. The current relay URL is prefilled when one is configured globally.</p>' +
      '<label>Upstream SMTP URL<input type="url" id="release-url" required value="' + escapeHtml((state.relay && state.relay.url) || '') + '" placeholder="smtp://relay.example.com:25" /></label>' +
      '<label class="checkbox"><input type="checkbox" id="release-insecure" /><span>Skip TLS certificate verification (dev only)</span></label>' +
      '<div class="release-actions"><button type="submit" class="btn">Send copy</button></div>' +
      '<div id="release-status"></div>';
    paneRelease.replaceChildren(form);
    form.addEventListener('submit', async (e) => {
      e.preventDefault();
      const url = paneRelease.querySelector('#release-url').value;
      const insecure = paneRelease.querySelector('#release-insecure').checked;
      const status = paneRelease.querySelector('#release-status');
      status.className = 'release-status';
      status.textContent = 'Sending…';
      const res = await api.release(m.id, url, insecure);
      if (res.ok) {
        status.className = 'release-status';
        status.textContent = 'Sent to ' + url + '.';
      } else {
        const body = await res.json().catch(() => ({}));
        status.className = 'release-status error';
        status.textContent = 'Failed: ' + (body.reason || res.statusText);
      }
    });

    setActiveTab(state.activeTab);
  };

  const setActiveTab = (name) => {
    state.activeTab = name;
    $$('.tab').forEach((t) => {
      const active = t.dataset.tab === name;
      t.classList.toggle('active', active);
      t.setAttribute('aria-selected', active ? 'true' : 'false');
    });
    $$('.pane').forEach((p) => p.classList.remove('active'));
    const pane = $('#pane-' + name);
    if (pane) pane.classList.add('active');
  };

  const select = (id) => {
    state.selectedId = id;
    renderList();
    renderDetail();
  };

  const renderRelayPill = () => {
    const pill = $('#relay-pill');
    const code = $('#relay-url-display');
    if (state.relay && state.relay.enabled) {
      pill.classList.add('active');
      code.textContent = state.relay.host + ':' + state.relay.port;
    } else {
      pill.classList.remove('active');
      code.textContent = 'off';
    }
  };
  const refreshRelay = async () => {
    state.relay = await api.relay.get();
    renderRelayPill();
  };
  const openRelayDialog = () => {
    const dlg = $('#relay-dialog');
    $('#relay-input-url').value = (state.relay && state.relay.enabled) ? state.relay.url : '';
    $('#relay-input-insecure').checked = !!(state.relay && state.relay.insecure);
    $('#relay-error').hidden = true;
    dlg.showModal();
  };
  const initRelayDialog = () => {
    $('#relay-pill').addEventListener('click', openRelayDialog);
    $('#relay-cancel').addEventListener('click', () => $('#relay-dialog').close());
    $('#relay-disable').addEventListener('click', async () => {
      await api.relay.del();
      await refreshRelay();
      $('#relay-dialog').close();
      toast('Relay disabled');
    });
    $('#relay-form').addEventListener('submit', async (e) => {
      e.preventDefault();
      const url = $('#relay-input-url').value;
      const insecure = $('#relay-input-insecure').checked;
      const res = await api.relay.put(url, insecure);
      if (res.ok) {
        await refreshRelay();
        $('#relay-dialog').close();
        toast('Relay configured');
      } else {
        const body = await res.json().catch(() => ({}));
        const err = $('#relay-error');
        err.textContent = body.reason || ('Error: ' + res.status);
        err.hidden = false;
      }
    });
  };

  let es = null;
  const setStatus = (kind) => {
    const dot = $('#status');
    dot.classList.remove('connected', 'disconnected');
    if (kind) dot.classList.add(kind);
    dot.title = kind === 'connected' ? 'Connected' : 'Disconnected';
  };
  const connect = () => {
    if (es) es.close();
    es = new EventSource('/api/stream');
    es.addEventListener('hello', () => setStatus('connected'));
    es.addEventListener('message', (ev) => {
      if (state.paused) return;
      const m = JSON.parse(ev.data);
      state.messages.unshift(m);
      if (state.messages.length > 5000) state.messages = state.messages.slice(0, 5000);
      renderList();
    });
    es.addEventListener('cleared', () => {
      state.messages = [];
      state.selectedId = null;
      renderList();
      renderDetail();
    });
    es.addEventListener('deleted', (ev) => {
      const id = JSON.parse(ev.data).id;
      state.messages = state.messages.filter((m) => m.id !== id);
      if (state.selectedId === id) state.selectedId = null;
      renderList();
      renderDetail();
    });
    es.addEventListener('resync', async () => {
      state.messages = await api.list();
      renderList();
    });
    es.onerror = () => {
      setStatus('disconnected');
      setTimeout(() => connect(), 1500);
    };
  };

  const boot = async () => {
    initTheme();
    const health = await api.health().catch(() => null);
    if (health) {
      $('#version').textContent = 'v' + health.version;
      if (health.smtp_port != null) {
        $('#smtp-url').textContent = 'smtp://' + location.hostname + ':' + health.smtp_port;
        $('#swaks-snippet').textContent =
          'swaks --to dev@example.com --from app@example.com \\\n' +
          '  --server ' + location.hostname + ':' + health.smtp_port + ' \\\n' +
          '  --header "Subject: Hello from MailBoxUltra" \\\n' +
          '  --body "It works."';
      }
    }
    state.messages = await api.list();
    renderList();
    await refreshRelay();
    initEvents();
    initRelayDialog();
    connect();
  };

  const initEvents = () => {
    $('#message-list').addEventListener('click', (e) => {
      const li = e.target.closest('.message-item');
      if (li) select(li.dataset.id);
    });
    $$('.tab').forEach((t) => t.addEventListener('click', () => setActiveTab(t.dataset.tab)));
    $('#search').addEventListener('input', (e) => {
      state.filter = e.target.value;
      renderList();
    });
    $('#clear-btn').addEventListener('click', async () => {
      if (state.messages.length === 0) return;
      if (!confirm('Clear all ' + state.messages.length + ' captured messages?')) return;
      await api.clear();
    });
    $('#pause-toggle').addEventListener('click', () => {
      state.paused = !state.paused;
      $('#pause-toggle').classList.toggle('active', state.paused);
      $('#pause-toggle').querySelector('.btn-label').textContent = state.paused ? 'Resume' : 'Pause';
      if (!state.paused) api.list().then((m) => { state.messages = m; renderList(); });
    });
    $('#theme-toggle').addEventListener('click', toggleTheme);
    $('#shortcuts-btn').addEventListener('click', () => $('#help-dialog').showModal());
    $('#help-close').addEventListener('click', () => $('#help-dialog').close());
    $('#copy-url').addEventListener('click', () => {
      const url = $('#smtp-url').textContent;
      navigator.clipboard.writeText(url).then(() => toast('Copied SMTP URL'));
    });
    $('#copy-swaks').addEventListener('click', () => {
      navigator.clipboard.writeText($('#swaks-snippet').textContent).then(() => toast('Copied'));
    });
    document.addEventListener('keydown', onKey);
  };

  const onKey = (e) => {
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    const inField = e.target.matches('input, textarea, select, [contenteditable]');
    const dlgOpen = document.querySelector('dialog[open]');
    if (e.key === 'Escape') {
      if (dlgOpen) dlgOpen.close();
      else if (inField) e.target.blur();
      return;
    }
    if (e.key === '/' && !inField) { e.preventDefault(); $('#search').focus(); return; }
    if (inField || dlgOpen) return;
    if (e.key === 'X' && e.shiftKey) {
      e.preventDefault();
      $('#clear-btn').click();
      return;
    }
    if (e.shiftKey) return;
    switch (e.key) {
      case 'j': case 'ArrowDown': moveSelection(1); e.preventDefault(); break;
      case 'k': case 'ArrowUp': moveSelection(-1); e.preventDefault(); break;
      case 'g': moveSelection('first'); e.preventDefault(); break;
      case 'G': moveSelection('last'); e.preventDefault(); break;
      case '1': setActiveTab('html'); break;
      case '2': setActiveTab('text'); break;
      case '3': setActiveTab('headers'); break;
      case '4': setActiveTab('attachments'); break;
      case '5': setActiveTab('source'); break;
      case '6': setActiveTab('release'); break;
      case 'p': $('#pause-toggle').click(); break;
      case 't': toggleTheme(); break;
      case '?': $('#help-dialog').showModal(); break;
      case 'd': deleteSelected(); break;
    }
  };

  const moveSelection = (delta) => {
    const visible = state.messages.filter((m) => matchFilter(m, state.filter));
    if (visible.length === 0) return;
    if (delta === 'first') return select(visible[0].id);
    if (delta === 'last') return select(visible[visible.length - 1].id);
    const idx = visible.findIndex((m) => m.id === state.selectedId);
    let next = idx === -1 ? 0 : Math.max(0, Math.min(visible.length - 1, idx + delta));
    select(visible[next].id);
    const li = document.querySelector('.message-item[data-id="' + visible[next].id + '"]');
    if (li) li.scrollIntoView({ block: 'nearest' });
  };

  const deleteSelected = async () => {
    if (!state.selectedId) return;
    await api.delete(state.selectedId);
  };

  document.addEventListener('DOMContentLoaded', boot);
})();
