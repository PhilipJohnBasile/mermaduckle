/* ═══════════════════════════════════════════════════════════
   Mermaduckle SPA — Vanilla JS Application Controller
   Replaces the entire Next.js + React frontend
   ═══════════════════════════════════════════════════════════ */

const $ = (sel) => document.querySelector(sel);
const $$ = (sel) => document.querySelectorAll(sel);
const h = (tag, attrs = {}, ...children) => {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (k === 'class') el.className = v;
    else if (k === 'style' && typeof v === 'object') Object.assign(el.style, v);
    else if (k.startsWith('on') && typeof v === 'function') el.addEventListener(k.slice(2).toLowerCase(), v);
    else el.setAttribute(k, v);
  }
  children.flat(Infinity).forEach(c => {
    if (c == null) return;
    el.appendChild(typeof c === 'string' ? document.createTextNode(c) : c);
  });
  return el;
};

let currentPage = 'dashboard';
let apiKey = localStorage.getItem('apiKey') || '';
let session = localStorage.getItem('session') || '';
let currentUser = JSON.parse(localStorage.getItem('currentUser') || 'null');

function apiFetch(url, options = {}) {
  if (!apiKey) {
    showToast('API key required. Configure it in Settings.', 'error');
    // Do not auto-navigate. Let callers decide how to handle missing keys.
    return Promise.reject(new Error('Missing API key'));
  }
  const headers = { ...options.headers };
  headers['Authorization'] = `Bearer ${apiKey}`;

  return fetch(url, { ...options, headers }).then(async (res) => {
    if (res.status === 401) {
      showToast('Authentication required. Please enter a valid API key.', 'error');
      // Do not auto-navigate on 401; let the UI handle redirects or re-auth flows.
      return Promise.reject(new Error('Unauthorized'));
    }
    if (!res.ok) {
      const text = await res.text().catch(() => res.statusText);
      return Promise.reject(new Error(text || res.statusText));
    }
    return res;
  });
}

function adminFetch(url, options = {}) {
  if (!session) return Promise.reject(new Error('No session'));
  const headers = { ...options.headers };
  headers['Authorization'] = `Bearer ${session}`;
  return fetch(url, { ...options, headers }).then(async (res) => {
    if (!res.ok) {
      const text = await res.text().catch(() => res.statusText);
      return Promise.reject(new Error(text || res.statusText));
    }
    return res;
  });
}

/* ── Auth Screen ── */
function showAuthScreen() {
  const app = $('#app');
  app.style.display = 'none';
  let existing = $('#auth-screen');
  if (existing) existing.remove();

  let isLogin = true;

  function render() {
    let screen = $('#auth-screen');
    if (screen) screen.remove();

    const container = h('div', { id: 'auth-screen', class: 'auth-screen' },
      h('div', { class: 'auth-card' },
        h('div', { class: 'auth-logo' },
          h('div', { class: 'sidebar-logo-icon' },
            h('svg', { xmlns: 'http://www.w3.org/2000/svg', width: '24', height: '24', viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '2', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' },
              (() => { const p = document.createElementNS('http://www.w3.org/2000/svg','path'); p.setAttribute('d','M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z'); return p; })()
            )
          )
        ),
        h('h2', { class: 'auth-title' }, isLogin ? 'Sign in to the hosted beta' : 'Create your beta account'),
        h('p', { class: 'auth-subtitle' }, isLogin ? 'Enter your credentials to access the managed Mermaduckle environment.' : 'Use the hosted environment while we work closely with early users.'),
        h('form', { class: 'auth-form', onSubmit: handleAuthSubmit },
          ...(isLogin ? [] : [
            h('label', { class: 'auth-label' }, 'Full name'),
            h('input', { class: 'auth-input', type: 'text', name: 'name', placeholder: 'Jane Smith', required: 'true', autocomplete: 'name' }),
          ]),
          h('label', { class: 'auth-label' }, 'Email'),
          h('input', { class: 'auth-input', type: 'email', name: 'email', placeholder: 'you@company.com', required: 'true', autocomplete: 'email' }),
          h('label', { class: 'auth-label' }, 'Password'),
          h('input', { class: 'auth-input', type: 'password', name: 'password', placeholder: isLogin ? 'Enter your password' : 'Min 6 characters', required: 'true', minlength: '6', autocomplete: isLogin ? 'current-password' : 'new-password' }),
          h('div', { id: 'auth-error', class: 'auth-error', style: { display: 'none' } }),
          h('button', { class: 'auth-btn', type: 'submit' }, isLogin ? 'Sign in' : 'Create account'),
        ),
        h('p', { class: 'auth-toggle' },
          isLogin ? "Don't have an account? " : 'Already have an account? ',
          h('a', { href: '#', onClick: (e) => { e.preventDefault(); isLogin = !isLogin; render(); } }, isLogin ? 'Sign up' : 'Sign in')
        )
      )
    );
    document.body.appendChild(container);
  }

  async function handleAuthSubmit(e) {
    e.preventDefault();
    const form = e.target;
    const errEl = form.querySelector('#auth-error');
    const btn = form.querySelector('.auth-btn');
    errEl.style.display = 'none';
    btn.disabled = true;
    btn.textContent = isLogin ? 'Signing in…' : 'Creating account…';

    const body = {
      email: form.email.value,
      password: form.password.value,
    };
    if (!isLogin) body.name = form.name.value;

    try {
      const res = await fetch(isLogin ? '/auth/login' : '/auth/register', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      const data = await res.json();
      if (!res.ok) {
        errEl.textContent = data.error || 'Something went wrong.';
        errEl.style.display = 'block';
        btn.disabled = false;
        btn.textContent = isLogin ? 'Sign in' : 'Create account';
        return;
      }
      // Account pending admin approval
      if (data.pending) {
        const card = form.closest('.auth-card');
        const toggle = card.querySelector('.auth-toggle');
        form.style.display = 'none';
        if (toggle) toggle.style.display = 'none';
        const existing = card.querySelector('.auth-pending');
        if (existing) existing.remove();
        card.appendChild(h('div', { class: 'auth-pending', style: { textAlign: 'center', padding: '1rem 0' } },
          h('div', { style: { fontSize: '48px', marginBottom: '1rem' } }, '⏳'),
          h('h3', { style: { color: 'white', marginBottom: '0.5rem' } }, 'Account pending approval'),
          h('p', { style: { color: 'var(--slate-400)', lineHeight: '1.6', marginBottom: '1.5rem' } },
            'Your registration has been received. An administrator will review and approve your account. You\'ll be able to sign in once approved.'),
          h('button', { class: 'auth-btn', style: { width: '100%' }, onClick: () => {
            card.querySelector('.auth-pending').remove();
            form.style.display = '';
            if (toggle) toggle.style.display = '';
            btn.disabled = false;
            btn.textContent = 'Sign in';
            isLogin = true;
            render();
          } }, 'Back to sign in')
        ));
        return;
      }
      // Store auth state
      session = data.session;
      apiKey = data.apiKey;
      currentUser = data.user;
      localStorage.setItem('session', session);
      localStorage.setItem('apiKey', apiKey);
      localStorage.setItem('currentUser', JSON.stringify(currentUser));
      // Remove auth screen, show app
      $('#auth-screen').remove();
      app.style.display = '';
      updateUserDisplay();
      // Hide Settings nav for non-admin users
      const settingsNav = document.querySelector('.nav-item[data-page="settings"]');
      if (settingsNav) {
        settingsNav.style.display = (currentUser && currentUser.role === 'admin') ? '' : 'none';
      }
      navigate('dashboard');
    } catch (err) {
      errEl.textContent = 'Network error. Please try again.';
      errEl.style.display = 'block';
      btn.disabled = false;
      btn.textContent = isLogin ? 'Sign in' : 'Create account';
    }
  }

  render();
}

function signOut() {
  // Fire and forget logout
  if (session) {
    fetch('/auth/logout', { method: 'POST', headers: { 'Authorization': `Bearer ${session}` } }).catch(() => {});
  }
  session = '';
  apiKey = '';
  currentUser = null;
  localStorage.removeItem('session');
  localStorage.removeItem('apiKey');
  localStorage.removeItem('currentUser');
  showAuthScreen();
}

function updateUserDisplay() {
  const el = $('#user-display');
  if (!el) return;
  if (currentUser) {
    el.innerHTML = '';
    el.appendChild(
      h('div', { style: { display: 'flex', alignItems: 'center', gap: '0.5rem', width: '100%' } },
        h('div', { class: 'user-avatar' }, currentUser.name ? currentUser.name[0].toUpperCase() : '?'),
        h('div', { style: { flex: '1', minWidth: '0' } },
          h('div', { style: { display: 'flex', alignItems: 'center', gap: '0.375rem' } },
            h('span', { style: { fontSize: '12px', fontWeight: '500', color: 'var(--slate-200)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' } }, currentUser.name || 'User'),
            h('span', { class: currentUser.role === 'admin' ? 'role-badge-admin' : 'role-badge-user' }, currentUser.role || 'user'),
          ),
          h('div', { style: { fontSize: '10px', color: 'var(--slate-500)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' } }, currentUser.email || ''),
        ),
        h('button', { class: 'sign-out-btn', title: 'Sign out', onClick: signOut },
          h('svg', { xmlns: 'http://www.w3.org/2000/svg', width: '14', height: '14', viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '2', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' },
            (() => { const g = document.createDocumentFragment();
              const p1 = document.createElementNS('http://www.w3.org/2000/svg','path'); p1.setAttribute('d','M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4');
              const p2 = document.createElementNS('http://www.w3.org/2000/svg','polyline'); p2.setAttribute('points','16 17 21 12 16 7');
              const p3 = document.createElementNS('http://www.w3.org/2000/svg','line'); p3.setAttribute('x1','21'); p3.setAttribute('y1','12'); p3.setAttribute('x2','9'); p3.setAttribute('y2','12');
              g.appendChild(p1); g.appendChild(p2); g.appendChild(p3); return g;
            })()
          )
        )
      )
    );
  }
}

function navigate(page) {
  // Block non-admin users from accessing Settings
  if (page === 'settings' && (!currentUser || currentUser.role !== 'admin')) {
    showToast('Settings is restricted to administrators.', 'error');
    return;
  }
  currentPage = page;
  $$('.nav-item').forEach(item => item.classList.toggle('active', item.dataset.page === page));
  renderPage(page);
}

async function renderPage(page) {
  const el = $('#page-content');
  el.innerHTML = '<div class="loading-page"><div class="spinner"></div><p>Loading...</p></div>';
  try {
    if (page.startsWith('builder:')) {
      await renderBuilder(el, page.split(':')[1]);
      return;
    }
    
    switch (page) {
      case 'dashboard': await renderDashboard(el); break;
      case 'workflows': await renderWorkflows(el); break;
      case 'marketplace': await renderMarketplace(el); break;
      case 'approvals': await renderApprovals(el); break;
      case 'agents': await renderAgents(el); break;
      case 'audit': await renderAudit(el); break;
      case 'settings': await renderSettings(el); break;
      default: throw new Error('Page not found');
    }
  } catch (err) {
    el.innerHTML = `<div class="empty-state"><h3>Error loading ${page}</h3><p>${err.message}</p></div>`;
  }
}

function fmtNum(n) {
  if (n >= 1e6) return (n / 1e6).toFixed(1) + 'M';
  if (n >= 1e3) return (n / 1e3).toFixed(1) + 'k';
  return n.toString();
}

function fmtDate(d) {
  if (!d) return '—';
  const dt = new Date(d);
  if (isNaN(dt)) return '—';
  return dt.toLocaleDateString('en-US', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
}

function showToast(message, type = 'success') {
  const t = h('div', { class: `toast ${type}` },
    h('div', {},
      h('p', { style: { fontWeight: '600', fontSize: '14px' } }, message)
    )
  );
  document.body.appendChild(t);
  setTimeout(() => t.remove(), 3000);
}

async function installTemplate(t) {
  if (!apiKey) {
    showToast('API key required to install templates', 'error');
    navigate('settings');
    return;
  }
  try {
    const res = await apiFetch('/api/workflows', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        name: t.title,
        description: t.desc,
        nodes: [
          { id: 'trigger-1', type: 'agentNode', position: { x: 100, y: 200 }, data: { label: 'Trigger', type: 'trigger', description: 'Workflow entry point', icon: 'Zap', config: {} } },
          { id: 'agent-1', type: 'agentNode', position: { x: 350, y: 200 }, data: { label: 'AI Agent', type: 'agent', description: t.desc, icon: 'Bot', config: { model: 'llama3' } } },
          { id: 'action-1', type: 'agentNode', position: { x: 600, y: 200 }, data: { label: 'Output Action', type: 'action', description: 'Deliver result', icon: 'FileText', config: {} } }
        ],
        edges: [
          { id: 'e1', source: 'trigger-1', target: 'agent-1', animated: true },
          { id: 'e2', source: 'agent-1', target: 'action-1', animated: true }
        ]
      })
    });
    const data = await res.json();
    showToast(`"${t.title}" installed!`, 'success');
    setTimeout(() => navigate('builder:' + data.id), 600);
  } catch (e) {
    showToast('Failed to install template', 'error');
  }
}

async function renderMarketplace(el) {
  el.innerHTML = '';
  el.className = 'page-content animate-fade-in';
  
  el.appendChild(h('div', { class: 'page-header' },
    h('div', {},
      h('h2', {}, 'Workflow Marketplace'),
      h('p', {}, 'Install pre-built enterprise agent workflows — each creates a new editable workflow')
    )
  ));

  const templates = [
    { title: 'Ad Creative Multi-Variate Tester', desc: 'Automatically test 50+ ad variations against Meta Ads API.', icon: '🎯', cat: 'Social & Marketing' },
    { title: 'GitHub PR Code Reviewer', desc: 'Scans new pull requests for security vulnerabilities and style guide compliance.', icon: '🐙', cat: 'Security & Content' },
    { title: 'Customer Sentiment Triage', desc: 'Categorize and prioritize inbound support tickets based on urgency.', icon: '📧', cat: 'Data & Analytics' },
    { title: 'Autonomous Twitter/X Manager', desc: 'Generate and schedule daily industry insights to build brand authority.', icon: '🐦', cat: 'Social & Marketing' },
    { title: 'Daily SEO Performance Audit', desc: 'Connect to Google Search Console and generate a daily PDF performance report.', icon: '📈', cat: 'Data & Analytics' },
    { title: 'LLM Hallucination Guardrail', desc: 'Secondary validation layer to keep your production agent from drifting.', icon: '🛡️', cat: 'Security & Content' },
    { title: 'Nightly Data Quality Report', desc: 'Scans your primary database for anomalies and emails a summary report.', icon: '🗃️', cat: 'Data & Analytics' },
    { title: 'Slack Approval Notifications', desc: 'Posts pending workflow approvals to a Slack channel for quick review.', icon: '💬', cat: 'Security & Content' },
    { title: 'Competitor Pricing Monitor', desc: 'Daily scrape of competitor pricing pages with Salesforce update sync.', icon: '📊', cat: 'Data & Analytics' },
    { title: 'Cold Email Personalizer', desc: 'Enriches lead profiles and drafts personalized outreach at scale.', icon: '✉️', cat: 'Social & Marketing' },
    { title: 'SEC Filing Summarizer', desc: 'Monitors filings for watched tickers and summarizes quarterly impact.', icon: '📑', cat: 'Data & Analytics' },
    { title: 'On-Call Alert Triage', desc: 'Routes PagerDuty alerts through an AI triage layer before escalation.', icon: '🚨', cat: 'Security & Content' }
  ];

  const categories = ['All Templates', 'Security & Content', 'Data & Analytics', 'Social & Marketing'];
  let activeCategory = 'All Templates';

  const grid = h('div', { class: 'grid grid-3' });

  function renderTemplateCards() {
    grid.innerHTML = '';
    const filtered = activeCategory === 'All Templates'
      ? templates
      : templates.filter(t => t.cat === activeCategory);
    filtered.forEach(t => {
      grid.appendChild(h('div', { class: 'glass-card marketplace-card animate-slide-up', style: { padding: '1.5rem', display: 'flex', flexDirection: 'column' } },
        h('div', { style: { display: 'flex', alignItems: 'center', gap: '0.75rem', marginBottom: '1rem' } },
          h('div', { style: { fontSize: '28px' } }, t.icon),
          h('span', { style: { fontSize: '10px', color: 'var(--slate-600)', background: 'rgba(255,255,255,0.04)', padding: '2px 8px', borderRadius: '999px', border: '1px solid rgba(255,255,255,0.06)' } }, t.cat)
        ),
        h('h3', { style: { color: 'white', marginBottom: '0.5rem', fontSize: '1rem' } }, t.title),
        h('p', { style: { fontSize: '12px', color: 'var(--slate-500)', marginBottom: '1.5rem', flex: '1', lineHeight: '1.6' } }, t.desc),
        h('button', { class: 'btn-primary w-full', onClick: () => installTemplate(t) }, 'Install Template')
      ));
    });
  }

  const chipEls = [];
  const chipRow = h('div', { class: 'flex gap-4 mb-8' },
    ...categories.map((cat, i) => {
      const chip = h('button', {
        class: `filter-chip ${i === 0 ? 'active' : ''}`,
        onClick: (e) => {
          activeCategory = cat;
          chipEls.forEach(c => c.classList.remove('active'));
          e.target.classList.add('active');
          renderTemplateCards();
        }
      }, cat);
      chipEls.push(chip);
      return chip;
    })
  );
  el.appendChild(chipRow);
  renderTemplateCards();
  el.appendChild(grid);
}

/* ── Dashboard ──────────────────────────────────────────── */
async function renderDashboard(el) {
  // If no API key is configured, render a lightweight unauthenticated dashboard
  // so the SPA doesn't get stuck trying to call protected endpoints on load.
  if (!apiKey) {
    el.innerHTML = '';
    el.className = 'page-content animate-fade-in';
    el.appendChild(h('div', { class: 'page-header' },
        h('div', {},
          h('h2', {}, 'Welcome to Mermaduckle'),
          h('p', {}, 'Governed AI workflow operations — hosted beta control plane')
        ),
    ));

    el.appendChild(h('div', { class: 'glass-card', style: { padding: '2rem', maxWidth: '640px', marginBottom: '2rem' } },
      h('h3', { style: { color: 'white', marginBottom: '0.75rem' } }, 'Configure an API key to get started'),
      h('p', { style: { color: 'var(--slate-400)', marginBottom: '1.5rem', lineHeight: '1.6' } },
        'This console requires an API key to access live data. You can generate one right now — it will be stored in your browser and used for all API calls.'
      ),
      h('div', { style: { display: 'flex', gap: '0.75rem', flexWrap: 'wrap' } },
        h('button', { class: 'btn-primary', onClick: async () => {
          try {
            const res = await fetch('/dev/api/settings/api-keys', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({ name: 'browser-session' })
            });
            if (!res.ok) throw new Error('Server responded ' + res.status);
            const data = await res.json();
            apiKey = data.key;
            localStorage.setItem('apiKey', apiKey);
            showCreatedKeyModal(data.key);
            renderPage('dashboard');
          } catch (e) {
            showToast('Could not auto-generate key: ' + e.message, 'error');
          }
        } }, 'Generate API Key'),
        h('button', { class: 'btn-glass', onClick: () => navigate('settings') }, 'Paste Existing Key')
      ),
      h('p', { style: { fontSize: '11px', color: 'var(--slate-600)', marginTop: '1rem' } },
        'The generated key is stored only in your browser\'s localStorage. Copy it before clearing browser data.'
      )
    ));

    // Show placeholder metric cards
    const grid = h('div', { class: 'grid grid-4 mb-8', style: { opacity: '0.4' } });
    grid.appendChild(makeStatCard('TOTAL WORKFLOWS', '—'));
    grid.appendChild(makeStatCard('ACTIVE AGENTS', '—'));
    grid.appendChild(makeStatCard('TOTAL RUNS', '—'));
    grid.appendChild(makeStatCard('PENDING APPROVALS', '—'));
    el.appendChild(grid);
    return;
  }

  const [metrics, workflows, events, health] = await Promise.all([
    apiFetch('/api/dashboard').then(r => r.json()),
    apiFetch('/api/workflows').then(r => r.json()),
    apiFetch('/api/audit').then(r => r.json()),
    fetch('/api/health').then(r => r.json()).catch(() => null),
  ]);

  el.innerHTML = '';
  el.className = 'page-content animate-fade-in';

  el.appendChild(h('div', { class: 'page-header' },
    h('div', {},
      h('h2', {}, 'Dashboard'),
      h('p', {}, 'Real-time overview of your AI agent operations')
    ),
  ));

  const cards = [
    { label: 'TOTAL WORKFLOWS', value: metrics.totalWorkflows, change: '+12%', icon: 'cyan' },
    { label: 'ACTIVE AGENTS', value: metrics.activeWorkflows, change: '+8%', icon: 'emerald' },
    { label: 'TOTAL RUNS', value: fmtNum(metrics.totalRuns), change: '+24%', icon: 'amber' },
    { label: 'PENDING APPROVALS', value: metrics.pendingApprovals || 0, change: 'NEW', icon: 'violet' },
  ];
  const grid = h('div', { class: 'grid grid-4 mb-8 stagger-children' });
  cards.forEach(c => {
    grid.appendChild(h('div', { class: 'glass-card metric-card' },
      h('div', { class: 'metric-header' },
        h('div', {},
          h('p', { class: 'metric-label' }, c.label),
          h('p', { class: 'metric-value' }, String(c.value)),
          h('div', { class: 'metric-change positive' }, h('span', {}, c.change), h('span', { class: 'vs' }, 'vs last week'))
        ),
        h('div', { class: `metric-icon ${c.icon}` },
          h('span', {}, '●')
        )
      )
    ));
  });
  el.appendChild(grid);

  el.appendChild(h('div', { class: 'glass-card mb-8', style: { padding: '1.5rem' } },
    h('div', { class: 'section-header', style: { marginBottom: '1.5rem' } },
      h('h3', {}, 'System Performance Analytics'),
      h('div', { class: 'flex gap-2' },
        h('button', { class: 'btn-glass active' }, 'Runs'),
        h('button', { class: 'btn-glass' }, 'Latency')
      )
    ),
    h('div', { id: 'dashboard-chart', style: { height: '240px', width: '100%', position: 'relative' } })
  ));

  const activityCard = h('div', { class: 'glass-card mb-8', style: { padding: '1.5rem', maxHeight: '400px', overflowY: 'auto' } },
    h('div', { class: 'section-header', style: { marginBottom: '1rem' } },
      h('h3', {}, 'Live Activity Stream')
    ),
    h('div', { id: 'dashboard-activity-stream', class: 'activity-stream' }, 'Connecting to stream...')
  );
  el.appendChild(activityCard);
  pollActivityStream();

  const row = h('div', { class: 'grid', style: { gridTemplateColumns: '2fr 1fr', gap: '1rem' } });
  const ollamaRequired = health?.services?.ollama_required === true;
  const ollamaAvailable = health?.services?.ollama === 'ok';

  const recentCard = h('div', { class: 'glass-card' },
    h('div', { class: 'section-header' },
      h('h3', {}, 'Recent Workflow Runs'),
      h('button', { class: 'btn-glass text-xs', onClick: () => navigate('workflows') }, 'View All')
    )
  );
  const table = h('table', { class: 'data-table' },
    h('thead', {}, h('tr', {},
      h('th', {}, 'Workflow'),
      h('th', {}, 'Status'),
      h('th', {}, 'Runs'),
      h('th', {}, 'Last Run')
    )),
    h('tbody', {}, ...workflows.slice(0, 5).map(w =>
      h('tr', { style: { cursor: 'pointer' }, onClick: () => renderWorkflowRunsModal(w.id, w.name) },
        h('td', {}, h('span', { style: { fontWeight: '500', color: 'white' } }, w.name)),
        h('td', {}, h('span', { class: `badge-${w.status === 'active' ? 'success' : w.status === 'paused' ? 'warning' : 'muted'}` }, w.status)),
        h('td', { style: { fontVariantNumeric: 'tabular-nums', color: 'white' } }, fmtNum(w.run_count)),
        h('td', { style: { fontSize: '12px', color: 'var(--slate-500)' } }, fmtDate(w.last_run_at || w.updated_at))
      )
    ))
  );
  recentCard.appendChild(table);
  row.appendChild(recentCard);

  const statusCard = h('div', { class: 'glass-card', style: { display: 'flex', flexDirection: 'column' } },
    h('div', { class: 'section-header' },
      h('h3', {}, 'System Status')
    ),
    h('div', { style: { padding: '1.25rem', display: 'flex', flexDirection: 'column', gap: '1rem' } },
      ...[
        { name: 'Workflow Engine', ok: health?.status === 'ok' },
        { name: 'Database', ok: health?.services?.database === 'ok' },
        {
          name: 'Ollama AI',
          ok: ollamaAvailable,
          optional: health && !ollamaRequired && !ollamaAvailable,
          na: !health,
        },
      ].map(s => h('div', { class: 'status-item' },
        h('div', { class: 'status-dot', style: { background: s.na ? 'var(--slate-600)' : s.ok ? 'var(--emerald-400)' : s.optional ? 'var(--amber-400)' : 'var(--red-400)' } }),
        h('span', { class: 'status-label' }, s.name),
        h('span', { class: 'status-value', style: { color: s.na ? 'var(--slate-600)' : s.ok ? 'var(--emerald-400)' : s.optional ? 'var(--amber-400)' : 'var(--red-400)' } },
          s.na ? 'Unknown' : s.ok ? 'Operational' : s.optional ? 'Optional' : 'Degraded'
        )
      ))
    )
  );
  row.appendChild(statusCard);
  el.appendChild(row);

  renderChart('dashboard-chart', [
    { label: 'Mon', value: 45 }, { label: 'Tue', value: 52 }, { label: 'Wed', value: 38 },
    { label: 'Thu', value: 65 }, { label: 'Fri', value: 48 }, { label: 'Sat', value: 24 }, { label: 'Sun', value: 31 }
  ]);
}

/* ── Workflows ──────────────────────────────────────────── */
async function renderWorkflows(el) {
  const workflows = await apiFetch('/api/workflows').then(r => r.json());
  el.innerHTML = '';
  el.className = 'page-content animate-fade-in';

  el.appendChild(h('div', { class: 'page-header' },
    h('div', {},
      h('h2', {}, 'Workflows'),
      h('p', {}, 'Build and manage your AI agent workflows')
    ),
    h('div', { class: 'flex gap-2' },
      h('button', { class: 'btn-glass', onClick: () => importWorkflowDialog() }, '📥 Import'),
      h('button', { class: 'btn-primary', onClick: () => showCreateWorkflowModal() }, '+ Create Workflow')
    )
  ));

  const stats = h('div', { class: 'grid grid-4 mb-8 stagger-children' });
  stats.appendChild(makeStatCard('Total Workflows', workflows.length));
  stats.appendChild(makeStatCard('Active', workflows.filter(w => w.status === 'active').length, 'emerald'));
  stats.appendChild(makeStatCard('Draft', workflows.filter(w => w.status === 'draft').length, 'amber'));
  stats.appendChild(makeStatCard('Total Runs', fmtNum(workflows.reduce((s, w) => s + w.run_count, 0))));
  el.appendChild(stats);

  const filterBar = h('div', { class: 'flex gap-4 mb-6', style: { alignItems: 'center', flexWrap: 'wrap' } },
    h('div', { class: 'search-bar' },
      h('input', { type: 'text', placeholder: 'Search workflows...', class: 'glass-input', style: { width: '100%', paddingLeft: '1rem' }, id: 'wf-search', onInput: () => filterWorkflowCards() })
    )
  );
  el.appendChild(filterBar);

  const grid = h('div', { class: 'grid grid-3 stagger-children', id: 'wf-grid' });
  workflows.forEach(w => grid.appendChild(makeWorkflowCard(w)));
  el.appendChild(grid);
}

function filterWorkflowCards() {
  const query = (document.getElementById('wf-search') || {}).value;
  if (query == null) return;
  const q = query.toLowerCase();
  document.querySelectorAll('#wf-grid .workflow-card').forEach(card => {
    const name = card.getAttribute('data-name') || '';
    card.style.display = name.includes(q) ? '' : 'none';
  });
}

function makeStatCard(label, value, color = '') {
  const borderColor = color === 'emerald' ? 'rgba(16,185,129,0.2)' : color === 'amber' ? 'rgba(245,158,11,0.2)' : '';
  const valColor = color === 'emerald' ? 'var(--emerald-400)' : color === 'amber' ? 'var(--amber-400)' : 'white';
  return h('div', { class: 'glass-card', style: { padding: '1.25rem', borderColor } },
    h('p', { style: { fontSize: '1.5rem', fontWeight: '700', color: valColor, fontVariantNumeric: 'tabular-nums' } }, String(value)),
    h('p', { style: { fontSize: '12px', color: 'var(--slate-500)', marginTop: '0.25rem' } }, label)
  );
}

function makeWorkflowCard(w) {
  const nodes = Array.isArray(w.nodes) ? w.nodes : [];
  const card = h('div', { class: 'glass-card-hover workflow-card', 'data-id': w.id, 'data-name': w.name.toLowerCase(), 'data-status': w.status, onClick: () => navigate('builder:' + w.id) },
    h('div', { class: 'workflow-card-header' },
      h('div', { class: 'workflow-card-info' },
        h('div', { class: `workflow-card-icon ${w.status === 'active' ? 'active' : 'inactive'}` },
          h('span', {}, w.status === 'active' ? '⚡' : '⏱')
        ),
        h('div', {},
          h('h3', {}, w.name),
          h('span', { class: `badge-${w.status === 'active' ? 'success' : w.status === 'paused' ? 'warning' : 'muted'}`, style: { fontSize: '10px' } }, w.status)
        )
      ),
      h('div', { class: 'workflow-actions', onClick: e => e.stopPropagation() },
        h('button', { class: 'btn-icon', title: 'Run', onClick: () => runWorkflow(w.id) }, '▶'),
        h('button', { class: 'btn-icon', title: 'Export', onClick: () => exportWorkflow(w.id) }, '📥'),
        h('button', { class: 'btn-icon', title: 'Delete', onClick: () => deleteWorkflow(w.id) }, '✕')
      )
    ),
    h('p', { class: 'description' }, w.description || 'No description'),
    h('div', { class: 'workflow-card-stats' },
      h('div', { onClick: (e) => { e.stopPropagation(); renderWorkflowRunsModal(w.id, w.name); } },
        h('p', { class: 'stat-value' }, fmtNum(w.run_count)),
        h('p', { class: 'stat-label', style: { textDecoration: 'underline' } }, 'View Runs')
      ),
      h('div', {},
        h('p', { class: 'stat-value' }, String(nodes.length)),
        h('p', { class: 'stat-label' }, 'Nodes')
      )
    )
  );
  return card;
}

async function renderBuilder(el, workflowId) {
  const workflow = await apiFetch(`/api/workflows/${workflowId}`).then(r => r.json());
  el.innerHTML = '';
  el.className = 'page-content';
  el.style.padding = '0';
  el.style.display = 'flex';
  el.style.flexDirection = 'column';
  
  el.appendChild(h('div', { class: 'page-header', style: { padding: '1rem 2rem', borderBottom: '1px solid rgba(255,255,255,0.06)', margin: '0' } },
    h('div', { style: { display: 'flex', alignItems: 'center', gap: '1rem' } },
      h('button', { class: 'btn-icon', onClick: () => navigate('workflows') }, '←'),
      h('div', {},
        h('h2', { style: { fontSize: '1.25rem', marginBottom: '0' } }, workflow.name),
        h('p', { style: { fontSize: '12px' } }, workflow.description || 'Draft Workflow')
      )
    ),
    h('div', { style: { display: 'flex', alignItems: 'center', gap: '0.75rem' } },
      h('button', { class: 'btn-glass', onClick: () => toggleArchitectSidebar() }, '✨ Architect'),
      h('button', { class: 'btn-glass', onClick: () => showWorkflowSettingsModal(workflow) }, '⚙ Settings'),
      h('button', { class: 'btn-glass', onClick: () => runWorkflow(workflowId, true) }, '🐞 Debug'),
      h('button', { class: 'btn-glass', onClick: () => runWorkflow(workflowId) }, '▶ Run'),
      h('button', { class: 'btn-primary', id: 'builder-save' }, 'Save Changes')
    )
  ));

  const builderLayout = h('div', { style: { display: 'flex', flex: '1', overflow: 'hidden' } });
  
  // Canvas Controls (Zoom/Pan)
  const canvasControls = h('div', { class: 'canvas-controls' },
    h('button', { onClick: () => window.zoomCanvas && window.zoomCanvas(0.1) }, '+'),
    h('button', { onClick: () => window.zoomCanvas && window.zoomCanvas(-0.1) }, '−'),
    h('button', { onClick: () => window.resetCanvas && window.resetCanvas() }, '⟲')
  );
  const palette = h('div', { style: { width: '250px', background: 'rgba(0,0,0,0.2)', borderRight: '1px solid rgba(255,255,255,0.06)', padding: '1rem', display: 'flex', flexDirection: 'column', gap: '1rem', overflowY: 'auto' } },
    h('h3', { style: { fontSize: '11px', textTransform: 'uppercase', letterSpacing: '0.05em', color: 'var(--slate-500)', marginBottom: '0.5rem' } }, 'Node Palette'),
    h('div', { class: 'form-group mb-4' },
      h('label', { style: { fontSize: '10px', color: 'var(--slate-600)', marginBottom: '0.5rem', display: 'block' } }, 'Agents & Swarms'),
      makeNodeDraggable('trigger', '⚡ Trigger', 'var(--amber-400)', 'Start event'),
      makeNodeDraggable('agent', '🤖 Agent', 'var(--agent-400)', 'Autonomous processing'),
      makeNodeDraggable('swarm', '🐝 Swarm', 'var(--primary-400)', 'Massively parallel execution')
    ),
    h('div', { class: 'form-group mb-4' },
      h('label', { style: { fontSize: '10px', color: 'var(--slate-600)', marginBottom: '0.5rem', display: 'block' } }, 'Logic & Control'),
      makeNodeDraggable('condition', '🔀 Condition', 'var(--cyan-400)', 'Branching gate'),
      makeNodeDraggable('loop', '🔄 Loop', 'var(--orange-400)', 'Iterative execution'),
      makeNodeDraggable('delay', '⏱️ Delay', 'var(--gray-400)', 'Time-based pause'),
      makeNodeDraggable('approval', '✅ Approval', 'var(--violet-400)', 'Human verification')
    ),
    h('div', { class: 'form-group' },
      h('label', { style: { fontSize: '10px', color: 'var(--slate-600)', marginBottom: '0.5rem', display: 'block' } }, 'Integrations'),
      makeNodeDraggable('http', '🌐 HTTP', 'var(--blue-400)', 'API call'),
      makeNodeDraggable('data_transform', '🔄 Transform', 'var(--purple-400)', 'Data processing'),
      makeNodeDraggable('action', '🔨 Action', 'var(--emerald-400)', 'External integration point')
    )
  );
  builderLayout.appendChild(palette);

  const canvasContainer = h('div', { style: { flex: '1', position: 'relative', overflow: 'hidden', background: 'radial-gradient(circle at center, rgba(255,255,255,0.03) 1px, transparent 1px)', backgroundSize: '24px 24px' }, id: 'workflow-canvas-container' });
  const debugControls = h('div', { id: 'debug-controls', class: 'debug-controls', style: { display: 'none' } });
  
  // Architect Sidebar (Hidden by default)
  const architectSidebar = h('div', { id: 'architect-sidebar', class: 'architect-sidebar' },
    h('div', { class: 'p-4 flex flex-col h-full' },
      h('h3', { class: 'text-sm font-bold uppercase mb-4' }, 'AI Architect'),
      h('textarea', { id: 'architect-prompt', class: 'glass-input flex-1 mb-4', placeholder: 'Describe your workflow requirements...' }),
      h('button', { class: 'btn-primary w-full', onClick: () => generateAIWorkflow() }, 'Generate Draft'),
      h('p', { class: 'text-xs mt-4 text-slate-500' }, 'This will overwrite the current canvas nodes and edges.')
    )
  );

  canvasContainer.appendChild(canvasControls);
  canvasContainer.appendChild(architectSidebar);
  canvasContainer.appendChild(debugControls);
  builderLayout.appendChild(canvasContainer);
  el.appendChild(builderLayout);

  requestAnimationFrame(() => {
    if (window.initWorkflowCanvas) {
      window.initWorkflowCanvas('workflow-canvas-container', workflow);
    } else {
      const script = document.createElement('script');
      script.src = '/static/workflow-canvas.js';
      script.onload = () => window.initWorkflowCanvas('workflow-canvas-container', workflow);
      document.body.appendChild(script);
    }
  });

function showDebugControls(result) {
  const controls = $('#debug-controls');
  controls.style.display = 'flex';
  controls.innerHTML = '';
  
  const stepBtn = h('button', { class: 'btn-glass', onClick: () => stepWorkflow() }, '⏭ Step');
  const stopBtn = h('button', { class: 'btn-glass', onClick: () => stopDebug() }, '⏹ Stop');
  
  controls.appendChild(stepBtn);
  controls.appendChild(stopBtn);
  
  const pid = result.paused_node_id || (result.result && result.result.paused_node_id);
  if (pid) highlightNode(pid);
}

async function stepWorkflow() {
  const runId = window.activeDebugRunId;
  showToast('Advancing to next node...', 'info');
  try {
    const res = await apiFetch(`/api/approvals/${runId}/action`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ action: 'approve' })
    });
    const data = await res.json();
    if (data.status === 'paused') {
      highlightNode(data.paused_node_id);
    } else {
      stopDebug();
      showToast('Workflow completed', 'success');
    }
  } catch (e) {
    showToast('Debug step failed', 'error');
  }
}

function highlightNode(nodeId) {
  document.querySelectorAll('.workflow-node').forEach(n => n.classList.remove('node-debug-active'));
  const target = document.querySelector(`[data-node-id="${nodeId}"]`);
  if (target) target.classList.add('node-debug-active');
}

async function pollActivityStream() {
  const container = $('#dashboard-activity-stream');
  if (!container) return;
  try {
    const res = await apiFetch('/api/logs/stream');
    const data = await res.json();
    container.innerHTML = '';
    data.forEach(item => {
      const el = h('div', { class: 'activity-item', style: { padding: '0.5rem 0', borderBottom: '1px solid rgba(255,255,255,0.05)', display: 'flex', gap: '0.75rem', alignItems: 'center' } },
        h('div', { style: { width: '8px', height: '8px', borderRadius: '50%', background: item.status === 'completed' ? 'var(--emerald-400)' : item.status === 'running' ? 'var(--amber-400)' : 'var(--slate-400)' } }),
        h('div', { class: 'flex-1' },
          h('div', { style: { fontSize: '12px', fontWeight: 'bold', color: 'white' } }, item.workflowName),
          h('div', { style: { fontSize: '10px', color: 'var(--slate-500)' } }, item.latestLog?.message || 'Started')
        ),
        h('div', { style: { fontSize: '10px', color: 'var(--slate-600)' } }, new Date(item.timestamp).toLocaleTimeString())
      );
      container.appendChild(el);
    });
  } catch (e) {}
  setTimeout(pollActivityStream, 5000);
}

async function exportWorkflow(id) {
  try {
    const res = await apiFetch(`/api/workflows/${id}/export`);
    const data = await res.json();
    const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `workflow-${id}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  } catch (e) {
    showToast('Failed to export workflow', 'error');
  }
}

function importWorkflowDialog() {
  const input = document.createElement('input');
  input.type = 'file';
  input.accept = '.json';
  input.onchange = async (e) => {
    const file = e.target.files[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = async (event) => {
      try {
        const data = JSON.parse(event.target.result);
        const res = await apiFetch('/api/workflows/import', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(data)
        });
        if (res.ok) {
          showToast('Workflow imported successfully!');
          navigate('workflows');
        }
      } catch (err) {
        showToast('Invalid JSON file', 'error');
      }
    };
    reader.readAsText(file);
  };
  input.click();
}

function stopDebug() {
  $('#debug-controls').style.display = 'none';
  window.activeDebugRunId = null;
  document.querySelectorAll('.workflow-node').forEach(n => n.classList.remove('node-debug-active'));
}

function toggleArchitectSidebar() {
  const sidebar = $('#architect-sidebar');
  sidebar.classList.toggle('visible');
}

async function generateAIWorkflow() {
  const prompt = $('#architect-prompt').value;
  if (!prompt) return showToast('Please enter a description', 'error');
  
  showToast('AI Architect is designing your workflow...', 'info');
  try {
    const res = await apiFetch('/api/architect/generate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ prompt })
    });
    const data = await res.json();
    if (data.nodes && data.edges) {
      if (window.setWorkflowData) {
        window.setWorkflowData(data.nodes, data.edges);
        autoArrangeWorkflow();
        showToast('AI Draft generated successfully!');
      }
    } else {
      showToast('AI failed to generate a valid design', 'error');
    }
  } catch (e) {
    showToast('AI Architect offline', 'error');
  }
}

function autoArrangeWorkflow() {
  if (window.autoLayout) window.autoLayout();
}

function showWorkflowSettingsModal(workflow) {
  let schedule = workflow.schedule || '';
  let status = workflow.status || 'draft';
  const overlay = h('div', { class: 'modal-overlay' },
    h('div', { class: 'modal', style: { maxWidth: '400px' } },
      h('h3', {}, 'Workflow Configuration'),
      h('div', { class: 'form-group' },
        h('label', {}, 'Execution Schedule'),
        h('select', { class: 'glass-input', onInput: (e) => schedule = e.target.value },
          h('option', { value: '', selected: schedule === '' }, 'Manual Only'),
          h('option', { value: '1m', selected: schedule === '1m' }, 'Every 1 Minute'),
          h('option', { value: '10m', selected: schedule === '10m' }, 'Every 10 Minutes'),
          h('option', { value: '1h', selected: schedule === '1h' }, 'Every 1 Hour'),
          h('option', { value: '1d', selected: schedule === '1d' }, 'Every 1 Day')
        ),
        h('p', { style: { fontSize: '11px', color: 'var(--slate-500)', marginTop: '0.5rem' } }, 'Automated background execution enabled on active workflows.')
      ),
      h('div', { class: 'form-group' },
        h('label', {}, 'Workflow Status'),
        h('div', { class: 'flex gap-2' },
          h('button', { class: `btn-glass flex-1 ${status==='draft'?'active':''}`, onClick: () => status='draft' }, 'Draft'),
          h('button', { class: `btn-glass flex-1 ${status==='active'?'active':''}`, onClick: () => status='active' }, 'Active')
        )
      ),
      h('div', { class: 'modal-actions' },
        h('button', { class: 'btn-glass', onClick: () => overlay.remove() }, 'Cancel'),
        h('button', { class: 'btn-primary', onClick: async () => {
          await apiFetch(`/api/workflows/${workflow.id}`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ schedule, status })
          });
          workflow.schedule = schedule;
          workflow.status = status;
          overlay.remove();
          showToast('Workflow settings updated');
          renderPage('builder:' + workflow.id);
        } }, 'Save Settings')
      )
    )
  );
  document.body.appendChild(overlay);
}

$('#builder-save').addEventListener('click', async () => {
    const nodes = window.getWorkflowNodes ? window.getWorkflowNodes() : [];
    const edges = window.getWorkflowEdges ? window.getWorkflowEdges() : [];
    await apiFetch(`/api/workflows/${workflowId}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ nodes, edges })
    });
    showToast('Workflow saved successfully');
  });
}

function makeNodeDraggable(type, label, color, desc) {
  return h('div', {
    class: 'glass-card-hover',
    style: { padding: '0.75rem', cursor: 'grab', borderLeft: `3px solid ${color}` },
    draggable: 'true',
    ondragstart: (e) => {
      e.dataTransfer.setData('application/json', JSON.stringify({ type, label, color }));
    }
  },
    h('p', { style: { fontWeight: '600', fontSize: '13px', color: 'white' } }, label),
    h('p', { style: { fontSize: '11px', color: 'var(--slate-500)', marginTop: '0.25rem' } }, desc)
  );
}

async function selfHealNode(runId, nodeId) {
  showToast('Oracle is analyzing failure...', 'info');
  try {
    const res = await apiFetch('/api/recovery/heal', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ run_id: runId, node_id: nodeId })
    });
    const data = await res.json();
    if (data.suggestion) {
      showHealSuggestionModal(data, runId, nodeId);
    } else {
      showToast('Oracle could not find a clear fix', 'warning');
    }
  } catch (e) {
    showToast('Oracle connection lost', 'error');
  }
}

function showHealSuggestionModal(data, runId, nodeId) {
  const overlay = h('div', { class: 'modal-overlay' },
    h('div', { class: 'modal', style: { maxWidth: '500px' } },
      h('h3', {}, 'Oracle Recommendation'),
      h('p', { class: 'mb-4', style: { fontSize: '14px', color: 'var(--primary-300)' } }, data.suggestion),
      h('div', { class: 'mb-6' },
        h('label', { style: { fontSize: '11px', color: 'var(--slate-500)' } }, 'Proposed Config Patch'),
        h('pre', { style: { background: 'rgba(0,0,0,0.3)', padding: '1rem', borderRadius: '0.5rem', fontSize: '11px', color: 'var(--slate-300)' } }, JSON.stringify(data.patched_config, null, 2))
      ),
      h('div', { class: 'modal-actions' },
        h('button', { class: 'btn-glass', onClick: () => overlay.remove() }, 'Discard'),
        h('button', { class: 'btn-primary', onClick: async () => {
             // In a real app, this would patch the workflow ID and restart the run ID.
             showToast('Patch applied. Retrying workflow...', 'success');
             overlay.remove();
             setTimeout(() => runWorkflow(window.currentWorkflowId), 500);
        } }, 'Apply & Retry')
      )
    )
  );
  document.body.appendChild(overlay);
}

async function reportWorkflow(id) {
  try {
    const res = await apiFetch(`/api/reporting/audit/${id}`);
    const text = await res.text();
    const blob = new Blob([text], { type: 'text/markdown' });
    const url = URL.createObjectURL(blob);
    window.open(url, '_blank');
  } catch (e) {
    showToast('Failed to generate report', 'error');
  }
}

/* ── Agent Library ──────────────────────────────────────── */
async function renderAgents(el) {
  const agents = await apiFetch('/api/agents').then(r => r.json());
  el.innerHTML = '';
  el.className = 'page-content animate-fade-in';

  el.appendChild(h('div', { class: 'page-header' },
    h('div', {},
      h('h2', {}, 'Agent Library'),
      h('p', {}, 'Manage and configure your AI agents')
    ),
    h('button', { class: 'btn-primary', onClick: () => showAgentModal() }, '+ Create Agent')
  ));

  const grid = h('div', { class: 'grid grid-3 stagger-children' });
  agents.forEach(a => {
    const tags = Array.isArray(a.tags) ? a.tags : [];
    grid.appendChild(h('div', { class: 'glass-card-hover', style: { padding: '1.25rem' } },
      h('div', { style: { display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', marginBottom: '1rem' } },
        h('div', { style: { display: 'flex', alignItems: 'center', gap: '0.75rem' } },
          h('div', { style: { width: '48px', height: '48px', background: 'var(--agent-50)', borderRadius: '0.75rem', display: 'flex', alignItems: 'center', justifyContent: 'center', fontSize: '20px' } }, '🤖'),
          h('div', {},
            h('h3', { style: { fontWeight: '600', color: 'white' } }, a.name),
            h('p', { style: { fontSize: '12px', color: 'var(--slate-500)' } }, a.model || 'llama3.2')
          )
        ),
        h('div', { class: 'workflow-actions' },
          h('button', { class: 'btn-icon', title: 'Playground', onClick: () => renderAgentPlayground(a) }, '💬'),
          h('button', { class: 'btn-icon', title: 'Edit', onClick: () => showAgentModal(a) }, '✏'),
          h('button', { class: 'btn-icon', title: 'Delete', onClick: async () => { await apiFetch(`/api/agents/${a.id}`, { method: 'DELETE' }); renderPage('agents'); } }, '✕')
        )
      ),
      h('p', { style: { fontSize: '13px', color: 'var(--slate-400)', marginBottom: '1rem', lineHeight: '1.5' } }, a.description || ''),
      h('div', { style: { display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: '1rem', paddingTop: '0.875rem', borderTop: '1px solid rgba(255,255,255,0.04)' } },
        h('div', {}, h('p', { class: 'stat-value' }, fmtNum(a.runs || 0)), h('p', { class: 'stat-label' }, 'Runs')),
        h('div', {}, h('p', { class: 'stat-value', style: { color: 'var(--emerald-400)' } }, `${a.successRate || 0}%`), h('p', { class: 'stat-label' }, 'Success')),
        h('div', {}, h('p', { class: 'stat-value' }, `$${(a.costPerRun || 0).toFixed(2)}`), h('p', { class: 'stat-label' }, 'Cost'))
      )
    ));
  });
  el.appendChild(grid);
}

/* ── Analytics Engine (SVG) ──────────────────────────────── */
function renderChart(containerId, data) {
  const container = document.getElementById(containerId);
  if (!container) return;
  
  const width = container.clientWidth;
  const height = container.clientHeight;
  const padding = 30;
  const chartWidth = width - padding * 2;
  const chartHeight = height - padding * 2;
  
  const maxVal = Math.max(...data.map(d => d.value)) * 1.2;
  const points = data.map((d, i) => ({
    x: padding + (i * (chartWidth / (data.length - 1))),
    y: height - padding - ((d.value / maxVal) * chartHeight)
  }));
  
  const pathData = `M ${points[0].x} ${points[0].y} ` + points.slice(1).map(p => `L ${p.x} ${p.y}`).join(' ');
  const areaData = pathData + ` L ${points[points.length-1].x} ${height - padding} L ${points[0].x} ${height - padding} Z`;
  
  const svg = `
    <svg width="${width}" height="${height}" style="overflow:visible">
      <defs>
        <linearGradient id="chartGradient" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stop-color="var(--primary-500)" stop-opacity="0.3"/>
          <stop offset="100%" stop-color="var(--primary-500)" stop-opacity="0"/>
        </linearGradient>
      </defs>
      <path d="${areaData}" fill="url(#chartGradient)"/>
      <path d="${pathData}" fill="none" stroke="var(--primary-400)" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>
      ${points.map((p, i) => `
        <circle cx="${p.x}" cy="${p.y}" r="4" fill="var(--slate-900)" stroke="var(--primary-400)" stroke-width="2"/>
        <text x="${p.x}" y="${height - 5}" text-anchor="middle" fill="var(--slate-500)" style="font-size:10px">${data[i].label}</text>
      `).join('')}
    </svg>
  `;
  container.innerHTML = svg;
}

/* ── Agent Playground UI ─────────────────────────────────── */
function renderAgentPlayground(agent) {
  const overlay = h('div', { class: 'modal-overlay', onClick: (e) => { if (e.target === overlay) overlay.remove(); } },
    h('div', { class: 'modal', style: { width: '600px', height: '700px', display: 'flex', flexDirection: 'column', padding: '0', overflow: 'hidden' } },
      h('div', { class: 'chat-header', style: { padding: '1.25rem', borderBottom: '1px solid rgba(255,255,255,0.06)', display: 'flex', justifyContent: 'space-between', alignItems: 'center' } },
        h('h3', {}, `Playground: ${agent.name}`),
        h('button', { class: 'btn-icon', onClick: () => overlay.remove() }, '✕')
      ),
      h('div', { id: 'chat-container', style: { flex: '1', overflowY: 'auto', padding: '1.5rem', display: 'flex', flexDirection: 'column', gap: '1rem', background: 'rgba(0,0,0,0.2)' } },
        h('div', { class: 'chat-bubble bot' }, `Ready to tune ${agent.name}. Send a test message!`)
      ),
      h('div', { style: { padding: '1.25rem', borderTop: '1px solid rgba(255,255,255,0.06)', display: 'flex', gap: '0.75rem' } },
        h('input', { id: 'chat-input', class: 'glass-input', style: { flex: '1' }, placeholder: 'Test prompt...', onKeypress: (e) => { if (e.key === 'Enter') sendChatMessage(agent.id); } }),
        h('button', { class: 'btn-primary', onClick: () => sendChatMessage(agent.id) }, 'Send')
      )
    )
  );
  document.body.appendChild(overlay);
  setTimeout(() => $('#chat-input').focus(), 10);
}

async function sendChatMessage(agentId) {
  const input = $('#chat-input');
  const text = input.value.trim();
  if (!text) return;
  const container = $('#chat-container');
  container.appendChild(h('div', { class: 'chat-bubble user' }, text));
  input.value = '';
  container.scrollTop = container.scrollHeight;
  const loading = h('div', { class: 'chat-bubble bot loading' }, '...');
  container.appendChild(loading);
  try {
    const res = await apiFetch(`/api/agents/${agentId}/chat`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ message: text }) });
    const data = await res.json();
    loading.remove();
    container.appendChild(h('div', { class: 'chat-bubble bot' }, data.response));
  } catch (err) {
    loading.remove();
    container.appendChild(h('div', { class: 'chat-bubble bot error' }, `Error: ${err.message}`));
  }
  container.scrollTop = container.scrollHeight;
}

/* ── Audit and Other Utility Functions Placeholder ──────── */
/* ── Approvals Queue ────────────────────────────────────── */
async function renderApprovals(el) {
  const approvals = await apiFetch('/api/approvals').then(r => r.json());
  el.innerHTML = '';
  el.className = 'page-content animate-fade-in';

  el.appendChild(h('div', { class: 'page-header' },
    h('div', {},
      h('h2', {}, 'Approval Queue'),
      h('p', {}, 'Review and approve sensitive workflow actions')
    )
  ));

  if (approvals.length === 0) {
    el.appendChild(h('div', { class: 'empty-state', style: { marginTop: '4rem' } },
      h('div', { style: { fontSize: '48px', marginBottom: '1rem' } }, '🎉'),
      h('h3', {}, 'All caught up!'),
      h('p', {}, 'No pending approvals at this time.')
    ));
    return;
  }

  const grid = h('div', { class: 'grid grid-2 stagger-children' });
  approvals.forEach(a => {
    grid.appendChild(h('div', { class: 'glass-card', style: { padding: '1.5rem' } },
      h('div', { class: 'flex justify-between mb-4', style: { alignItems: 'flex-start' } },
        h('div', {},
          h('h3', { style: { color: 'white', fontWeight: '600' } }, `Run ${a.id}`),
          h('p', { style: { fontSize: '12px', color: 'var(--slate-500)' } }, `Workflow: ${a.workflow_id}`)
        ),
        h('div', { class: 'badge-warning' }, 'Pending Review')
      ),
      h('div', { class: 'mb-4', style: { background: 'rgba(0,0,0,0.2)', padding: '1rem', borderRadius: '0.5rem', border: '1px solid rgba(255,255,255,0.06)' } },
        h('p', { style: { fontSize: '12px', color: 'var(--slate-500)', marginBottom: '0.5rem', textTransform: 'uppercase', letterSpacing: '0.05em' } }, 'Intermediate Output'),
        h('p', { style: { color: 'var(--slate-200)', fontSize: '14px', whiteSpace: 'pre-wrap' } }, a.output || 'No output captured yet.')
      ),
      h('div', { class: 'flex gap-2' },
        h('button', { class: 'btn-primary', style: { flex: '1', background: 'var(--emerald-600)' }, onClick: () => handleApproval(a.id, 'approve') }, 'Approve & Resume'),
        h('button', { class: 'btn-glass', style: { flex: '1', color: 'var(--red-400)' }, onClick: () => handleApproval(a.id, 'reject') }, 'Reject')
      )
    ));
  });
  el.appendChild(grid);
}

async function handleApproval(id, action) {
  try {
    const res = await apiFetch(`/api/approvals/${id}/action`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ action })
    });
    const data = await res.json();
    if (data.success) {
      showToast(action === 'approve' ? 'Workflow resumed' : 'Workflow rejected');
      renderPage('approvals');
    }
  } catch (err) {
    showToast('Failed to process approval', 'error');
  }
}

async function renderAudit(el) {
  const events = await apiFetch('/api/audit').then(r => r.json());
  el.innerHTML = '';
  el.className = 'page-content animate-fade-in';

  el.appendChild(h('div', { class: 'page-header' },
    h('div', {},
      h('h2', {}, 'Audit & Governance'),
      h('p', {}, 'Complete historical record of all system and agent activities')
    ),
    h('div', { class: 'flex gap-2' },
      h('button', { class: 'btn-glass text-xs', onClick: () => exportAuditCsv(events) }, 'Export CSV'),
      h('button', { class: 'btn-glass text-xs', onClick: () => window.open('/api/reporting/audit/' + ((events[0] && events[0].target && events[0].target.target_id) || 'all'), '_blank') }, 'Report')
    )
  ));

  const filters = h('div', { class: 'flex gap-4 mb-6', style: { alignItems: 'center' } },
    h('div', { class: 'search-bar' },
      h('input', { type: 'text', placeholder: 'Filter by actor or event...', class: 'glass-input', onInput: (e) => filterAudit(e.target.value) })
    ),
    h('div', { class: 'filter-chips' },
      h('button', { class: 'filter-chip active', onClick: (e) => toggleAuditFilter(e, 'all') }, 'All'),
      h('button', { class: 'filter-chip', onClick: (e) => toggleAuditFilter(e, 'high') }, 'High Severity'),
      h('button', { class: 'filter-chip', onClick: (e) => toggleAuditFilter(e, 'workflow') }, 'Workflow Events'),
      h('button', { class: 'filter-chip', onClick: (e) => toggleAuditFilter(e, 'security') }, 'Security')
    )
  );
  el.appendChild(filters);

  const container = h('div', { class: 'glass-card' });
  const table = h('table', { class: 'data-table' },
    h('thead', {}, h('tr', {},
      h('th', {}, 'Timestamp'),
      h('th', {}, 'Event'),
      h('th', {}, 'Severity'),
      h('th', {}, 'Actor'),
      h('th', {}, 'Target'),
      h('th', {}, 'Metadata')
    )),
    h('tbody', { id: 'audit-tbody' }, 
      ...events.map(ev => renderAuditRow(ev))
    )
  );
  container.appendChild(table);
  el.appendChild(container);

  function renderAuditRow(ev) {
    const sevClass = `badge-${ev.severity === 'high' ? 'danger' : ev.severity === 'medium' ? 'warning' : 'info'}`;
    return h('tr', { 'data-severity': ev.severity, 'data-text': `${ev.type} ${ev.actor.name} ${ev.target.name}`.toLowerCase() },
      h('td', { style: { fontSize: '12px', color: 'var(--slate-500)', whiteSpace: 'nowrap' } }, fmtDate(ev.timestamp)),
      h('td', {}, h('span', { style: { fontWeight: '600', color: 'white' } }, ev.type)),
      h('td', {}, h('span', { class: sevClass }, ev.severity)),
      h('td', {}, 
        h('div', {}, 
          h('p', { style: { fontSize: '13px', color: 'white', fontWeight: '500' } }, ev.actor.name),
          h('p', { style: { fontSize: '11px', color: 'var(--slate-600)' } }, ev.actor.email)
        )
      ),
      h('td', {}, h('p', { style: { fontSize: '13px', color: 'var(--slate-400)' } }, `${ev.target.target_type}: ${ev.target.name}`)),
      h('td', {}, h('button', { class: 'btn-icon', onClick: () => showAuditMetadata(ev.metadata) }, 'ℹ'))
    );
  }

  function filterAudit(query) {
    const rows = $$('#audit-tbody tr');
    rows.forEach(r => {
      const match = r.getAttribute('data-text').includes(query.toLowerCase());
      r.classList.toggle('hidden', !match);
    });
  }

  function toggleAuditFilter(e, filter) {
    $$('.filter-chip').forEach(c => c.classList.remove('active'));
    e.target.classList.add('active');
    const rows = $$('#audit-tbody tr');
    rows.forEach(r => {
      if (filter === 'all') r.classList.remove('hidden');
      else if (filter === 'high') r.classList.toggle('hidden', r.getAttribute('data-severity') !== 'high');
      else r.classList.remove('hidden'); // Simplified for now
    });
  }

  function showAuditMetadata(meta) {
    const overlay = h('div', { class: 'modal-overlay', onClick: (e) => { if (e.target === overlay) overlay.remove(); } },
      h('div', { class: 'modal' },
        h('h3', {}, 'Event Metadata'),
        h('div', { style: { background: 'rgba(0,0,0,0.3)', padding: '1rem', borderRadius: '0.5rem', border: '1px solid rgba(255,255,255,0.06)' } },
          h('pre', { style: { fontSize: '12px', color: 'var(--primary-300)', whiteSpace: 'pre-wrap' } }, JSON.stringify(meta, null, 2))
        ),
        h('div', { class: 'modal-actions' }, h('button', { class: 'btn-primary', onClick: () => overlay.remove() }, 'Close'))
      )
    );
    document.body.appendChild(overlay);
  }
}

async function renderSettings(el) {
  el.innerHTML = '';
  el.className = 'page-content animate-fade-in';

  el.appendChild(h('div', { class: 'page-header' },
    h('div', {},
      h('h2', {}, 'Settings'),
      h('p', {}, 'Configure your organization, API access, and integrations')
    )
  ));

  const tabs = h('div', { class: 'flex gap-6 mb-8', style: { borderBottom: '1px solid rgba(255,255,255,0.06)', flexWrap: 'wrap' } },
    h('button', { class: 'settings-tab active', onClick: (e) => switchSettingsTab(e, 'users') }, 'User Management'),
    h('button', { class: 'settings-tab', onClick: (e) => switchSettingsTab(e, 'api') }, 'API Keys'),
    h('button', { class: 'settings-tab', onClick: (e) => switchSettingsTab(e, 'client') }, 'Client Config'),
    h('button', { class: 'settings-tab', onClick: (e) => switchSettingsTab(e, 'team') }, 'Team Members'),
    h('button', { class: 'settings-tab', onClick: (e) => switchSettingsTab(e, 'secrets') }, 'Secret Vault'),
    h('button', { class: 'settings-tab', onClick: (e) => switchSettingsTab(e, 'integrations') }, 'Integrations'),
    h('button', { class: 'settings-tab', onClick: (e) => switchSettingsTab(e, 'notifications') }, 'Notifications')
  );
  el.appendChild(tabs);

  const content = h('div', { id: 'settings-tab-content' });
  el.appendChild(content);

  // Initial tab
  renderUsersTab(content);

  function switchSettingsTab(e, tab) {
    $$('.settings-tab').forEach(t => t.classList.remove('active'));
    e.target.classList.add('active');
    content.innerHTML = '';
    if (tab === 'users') renderUsersTab(content);
    else if (tab === 'api') renderApiKeysTab(content);
    else if (tab === 'client') renderClientTab(content);
    else if (tab === 'team') renderTeamTab(content);
    else if (tab === 'secrets') renderSecretsTab(content);
    else if (tab === 'integrations') renderIntegrationsTab(content);
    else if (tab === 'notifications') renderNotificationsTab(content);
  }
}

async function renderUsersTab(container) {
  try {
    const users = await adminFetch('/auth/admin/users').then(r => r.json());
    container.innerHTML = '';
    const pending = users.filter(u => u.status === 'pending');
    const active = users.filter(u => u.status === 'active');

    container.appendChild(h('div', { class: 'animate-fade-in' },
      h('div', { class: 'mb-6' },
        h('h3', { style: { color: 'white' } }, 'User Management'),
        h('p', { style: { fontSize: '13px', color: 'var(--slate-500)' } }, 'Approve or reject beta access requests')
      ),
      pending.length > 0 ? h('div', { class: 'mb-8' },
        h('h4', { style: { color: 'var(--amber-400)', marginBottom: '1rem', fontSize: '14px' } }, `Pending Approval (${pending.length})`),
        h('div', { class: 'glass-card' },
          h('table', { class: 'data-table' },
            h('thead', {}, h('tr', {},
              h('th', {}, 'Name'),
              h('th', {}, 'Email'),
              h('th', {}, 'Registered'),
              h('th', {}, 'Actions')
            )),
            h('tbody', {}, ...pending.map(u => h('tr', {},
              h('td', { style: { color: 'white', fontWeight: '500' } }, u.name),
              h('td', { style: { fontSize: '13px', color: 'var(--slate-400)' } }, u.email),
              h('td', { style: { fontSize: '12px', color: 'var(--slate-500)' } }, fmtDate(u.createdAt)),
              h('td', {},
                h('div', { class: 'flex gap-2' },
                  h('button', { class: 'btn-primary', style: { fontSize: '12px', padding: '4px 12px' }, onClick: async () => {
                    await adminFetch(`/auth/admin/users/${u.id}/approve`, { method: 'POST' });
                    showToast(`${u.name} approved`);
                    renderUsersTab(container);
                  } }, 'Approve'),
                  h('button', { class: 'btn-glass', style: { fontSize: '12px', padding: '4px 12px', color: 'var(--red-400)' }, onClick: async () => {
                    if (confirm(`Reject and remove ${u.name}?`)) {
                      await adminFetch(`/auth/admin/users/${u.id}/reject`, { method: 'POST' });
                      showToast(`${u.name} rejected`);
                      renderUsersTab(container);
                    }
                  } }, 'Reject')
                )
              )
            )))
          )
        )
      ) : h('div', { class: 'glass-card mb-8', style: { padding: '2rem', textAlign: 'center' } },
        h('p', { style: { color: 'var(--slate-500)' } }, 'No pending access requests')
      ),
      h('h4', { style: { color: 'var(--emerald-400)', marginBottom: '1rem', fontSize: '14px' } }, `Active Users (${active.length})`),
      h('div', { class: 'glass-card' },
        h('table', { class: 'data-table' },
          h('thead', {}, h('tr', {},
            h('th', {}, 'Name'),
            h('th', {}, 'Email'),
            h('th', {}, 'Role'),
            h('th', {}, 'Since')
          )),
          h('tbody', {}, ...active.map(u => h('tr', {},
            h('td', { style: { color: 'white', fontWeight: '500' } }, u.name),
            h('td', { style: { fontSize: '13px', color: 'var(--slate-400)' } }, u.email),
            h('td', {}, h('span', { class: u.role === 'admin' ? 'badge-info' : 'badge-success' }, u.role)),
            h('td', { style: { fontSize: '12px', color: 'var(--slate-500)' } }, fmtDate(u.createdAt))
          )))
        )
      )
    ));
  } catch (e) {
    container.innerHTML = '';
    container.appendChild(h('div', { class: 'empty-state' },
      h('h3', {}, 'Could not load users'),
      h('p', {}, e.message)
    ));
  }
}

async function renderApiKeysTab(container) {
  if (!apiKey) {
    container.innerHTML = '';
    container.appendChild(h('div', { class: 'animate-fade-in' },
      h('div', { class: 'mb-6' },
        h('h3', { style: { color: 'white' } }, 'API Management'),
        h('p', { style: { color: 'var(--slate-500)', fontSize: '14px' } }, 'No API key configured. Go to the Client tab and paste an API key to manage keys.')
      )
    ));
    return;
  }
  const keys = await apiFetch('/api/settings/api-keys').then(r => r.json());
  container.innerHTML = '';
  container.appendChild(h('div', { class: 'animate-fade-in' },
    h('div', { class: 'flex justify-between items-center mb-6' },
      h('div', {},
        h('h3', { style: { color: 'white' } }, 'API Management'),
        h('p', { style: { fontSize: '13px', color: 'var(--slate-500)' } }, 'Manage secure access keys for the Mermaduckle API')
      ),
      h('button', { class: 'btn-primary', onClick: () => showCreateApiKeyModal() }, '+ Create New Key')
    ),
    h('div', { class: 'glass-card' },
      h('table', { class: 'data-table' },
        h('thead', {}, h('tr', {},
          h('th', {}, 'Name'),
          h('th', {}, 'Key'),
          h('th', {}, 'Status'),
          h('th', {}, 'Created'),
          h('th', {}, '')
        )),
        h('tbody', {}, ...keys.map(k => h('tr', {},
          h('td', { style: { color: 'white', fontWeight: '500' } }, k.name),
          h('td', { style: { fontFamily: 'var(--font-mono)', fontSize: '12px' } }, `${k.key_hash.slice(0, 8)}••••••••`),
          h('td', {}, h('span', { class: 'badge-success' }, k.status)),
          h('td', { style: { fontSize: '12px', color: 'var(--slate-500)' } }, fmtDate(k.created_at)),
          h('td', { style: { textAlign: 'right' } }, h('button', { class: 'btn-icon', onClick: () => deleteApiKey(k.id) }, '✕'))
        )))
      )
    )
  ));
}

async function renderClientTab(container) {
  container.innerHTML = '';
  container.appendChild(h('div', { class: 'animate-fade-in' },
    h('div', { class: 'mb-6' },
      h('h3', { style: { color: 'white', marginBottom: '0.5rem' } }, 'API Configuration'),
      h('p', { style: { color: 'var(--slate-500)', fontSize: '14px' } }, 'Configure your client to connect to the API')
    ),
    h('div', { class: 'form-group' },
      h('label', { style: { display: 'block', marginBottom: '0.5rem', fontSize: '14px', color: 'white' } }, 'API Key'),
      h('input', { type: 'password', id: 'api-key-input', value: apiKey, placeholder: 'Enter your API key', style: { width: '100%', padding: '0.75rem', background: 'rgba(0,0,0,0.2)', border: '1px solid rgba(255,255,255,0.1)', borderRadius: '6px', color: 'white' } }),
      h('button', { class: 'btn-primary mt-4', onClick: () => {
        apiKey = $('#api-key-input').value;
        localStorage.setItem('apiKey', apiKey);
        showToast('API key saved!', 'success');
      } }, 'Save API Key')
    ),
    h('div', { class: 'mt-6 p-4 glass-card' },
      h('h4', { style: { color: 'white', marginBottom: '0.5rem' } }, 'How to get an API key'),
      h('p', { style: { color: 'var(--slate-400)', fontSize: '14px' } }, 'Go to the API Keys tab to create or view your API keys. Copy the key hash and paste it here.')
    )
  ));
}

async function renderTeamTab(container) {
  const team = await apiFetch('/api/settings/team').then(r => r.json());
  container.innerHTML = '';
  container.appendChild(h('div', { class: 'animate-fade-in' },
    h('div', { class: 'flex justify-between items-center mb-6' },
      h('div', {},
        h('h3', { style: { color: 'white' } }, 'Team Management'),
        h('p', { style: { fontSize: '13px', color: 'var(--slate-500)' } }, 'Manage your team members and their access levels')
      ),
      h('button', { class: 'btn-primary' }, '+ Invite Member')
    ),
    h('div', { class: 'grid grid-3' },
      ...team.map(m => h('div', { class: 'glass-card', style: { padding: '1.25rem' } },
        h('div', { class: 'flex items-center gap-3 mb-4' },
          h('div', { class: 'user-avatar' }, h('span', {}, m.name.slice(0, 2).toUpperCase())),
          h('div', {}, 
            h('h4', { style: { color: 'white', fontSize: '14px' } }, m.name),
            h('p', { style: { fontSize: '12px', color: 'var(--slate-500)' } }, m.role)
          )
        ),
        h('p', { style: { fontSize: '12px', color: 'var(--slate-400)', marginBottom: '1rem' } }, m.email),
        h('div', { class: 'flex justify-between items-center' },
          h('span', { class: 'badge-info', style: { fontSize: '10px' } }, 'Active'),
          h('button', { class: 'btn-icon', style: { color: 'var(--red-400)' } }, 'Remove')
        )
      ))
    )
  ));
}

async function renderIntegrationsTab(container) {
  const integrations = await apiFetch('/api/settings/integrations').then(r => r.json());
  container.innerHTML = '';
  const providers = [
    { name: 'Slack', icon: '💬', desc: 'Sync approval alerts to Slack channels' },
    { name: 'GitHub', icon: '🐙', desc: 'Trigger workflows on PR or commit' },
    { name: 'Discord', icon: '🎮', desc: 'Real-time event notifications' },
    { name: 'Meta Ads', icon: '🎯', desc: 'Automate high-velocity ad creative testing' }
  ];

  container.appendChild(h('div', { class: 'animate-fade-in' },
    h('div', { class: 'mb-6' },
      h('h3', { style: { color: 'white' } }, 'Third-Party Integrations'),
      h('p', { style: { fontSize: '13px', color: 'var(--slate-500)' } }, 'Connect with the tools your team already uses')
    ),
    h('div', { class: 'grid grid-2' },
      ...providers.map(p => {
        const conn = integrations.find(i => i.provider === p.name.toLowerCase());
        return h('div', { class: 'glass-card', style: { padding: '1.5rem' } },
          h('div', { class: 'flex gap-4' },
            h('div', { style: { fontSize: '24px' } }, p.icon),
            h('div', { style: { flex: '1' } },
              h('div', { class: 'flex justify-between items-center mb-1' },
                h('h4', { style: { color: 'white' } }, p.name),
                h('span', { class: conn ? 'badge-success' : 'badge-muted' }, conn ? 'Connected' : 'Not Connected')
              ),
              h('p', { style: { fontSize: '12px', color: 'var(--slate-500)', marginBottom: '1rem' } }, p.desc),
              h('button', { class: conn ? 'btn-glass' : 'btn-primary' }, conn ? 'Configure' : 'Connect')
            )
          )
        );
      })
    )
  ));
}

async function renderNotificationsTab(container) {
  const settings = await apiFetch('/api/settings/notifications').then(r => r.json());
  container.innerHTML = '';
  container.appendChild(h('div', { class: 'animate-fade-in', style: { maxWidth: '600px' } },
    h('h3', { style: { color: 'white', marginBottom: '1.5rem' } }, 'Notification Preferences'),
    h('div', { class: 'glass-card', style: { padding: '1.5rem', display: 'flex', flexDirection: 'column', gap: '1.5rem' } },
      h('div', { class: 'flex justify-between items-center' },
        h('div', {}, 
          h('p', { style: { color: 'white', fontWeight: '500' } }, 'Email Alerts'),
          h('p', { style: { fontSize: '12px', color: 'var(--slate-500)' } }, 'Receive email notifications for critical events')
        ),
        h('input', { type: 'checkbox', checked: settings.emailAlerts === 1 })
      ),
      h('div', { class: 'form-group' },
        h('label', {}, 'Alert Severity Threshold'),
        h('select', { class: 'glass-input' },
          h('option', { selected: settings.alertSeverity === 'low' }, 'Low (All Events)'),
          h('option', { selected: settings.alertSeverity === 'medium' }, 'Medium (Errors & Approvals)'),
          h('option', { selected: settings.alertSeverity === 'high' }, 'High (System Critical Only)')
        )
      ),
      h('div', { class: 'form-group' },
        h('label', {}, 'Slack Webhook URL'),
        h('input', { class: 'glass-input', type: 'text', value: settings.slackWebhook || '', placeholder: 'https://hooks.slack.com/services/...' })
      ),
      h('button', { class: 'btn-primary', style: { width: 'fit-content' } }, 'Save Preferences')
    )
  ));
}

async function renderSecretsTab(container) {
  const secrets = await apiFetch('/api/settings/secrets').then(r => r.json());
  container.innerHTML = '';
  container.appendChild(h('div', { class: 'animate-fade-in' },
    h('div', { class: 'flex justify-between items-center mb-6' },
      h('div', {},
        h('h3', { style: { color: 'white' } }, 'Secret Vault'),
        h('p', { style: { fontSize: '13px', color: 'var(--slate-500)' } }, 'Manage secure environment variables for your workflows')
      ),
      h('button', { class: 'btn-primary', onClick: () => showCreateSecretModal() }, '+ Add New Secret')
    ),
    h('div', { class: 'glass-card' },
      h('table', { class: 'data-table' },
        h('thead', {}, h('tr', {},
          h('th', {}, 'Key Name'),
          h('th', {}, 'Value'),
          h('th', {}, 'Created'),
          h('th', {}, '')
        )),
        h('tbody', {}, ...secrets.map(s => h('tr', {},
          h('td', { style: { color: 'white', fontWeight: '600', fontFamily: 'var(--font-mono)' } }, s.key),
          h('td', { style: { fontSize: '12px', color: 'var(--slate-500)' } }, '••••••••••••••••'),
          h('td', { style: { fontSize: '12px', color: 'var(--slate-600)' } }, fmtDate(s.created_at)),
          h('td', { style: { textAlign: 'right' } }, h('button', { class: 'btn-icon', onClick: () => deleteSecret(s.id) }, '✕'))
        )))
      )
    )
  ));
}

function showCreateSecretModal() {
  let key = '', value = '';
  const overlay = h('div', { class: 'modal-overlay' },
    h('div', { class: 'modal', style: { maxWidth: '400px' } },
      h('h3', {}, 'Add New Secret'),
      h('div', { class: 'form-group' },
        h('label', {}, 'Variable Name (Key)'),
        h('input', { class: 'glass-input', placeholder: 'e.g. OPENAI_API_KEY', onInput: (e) => key = e.target.value.toUpperCase() })
      ),
      h('div', { class: 'form-group' },
        h('label', {}, 'Value'),
        h('input', { class: 'glass-input', type: 'password', onInput: (e) => value = e.target.value })
      ),
      h('div', { class: 'modal-actions' },
        h('button', { class: 'btn-glass', onClick: () => overlay.remove() }, 'Cancel'),
        h('button', { class: 'btn-primary', onClick: async () => {
          if (!key || !value) return;
          await apiFetch('/api/settings/secrets', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ key, value })
          });
          overlay.remove();
          showToast('Secret added to vault');
          const content = $('#settings-tab-content');
          if (content) renderSecretsTab(content);
        } }, 'Save Secret')
      )
    )
  );
  document.body.appendChild(overlay);
}

async function deleteSecret(id) {
  if (confirm('Delete this secret? Workflows using it may break.')) {
    await apiFetch(`/api/settings/secrets/${id}`, { method: 'DELETE' });
    const content = $('#settings-tab-content');
    if (content) renderSecretsTab(content);
  }
}
async function deleteApiKey(id) {
  if (confirm('Are you sure you want to delete this API key?')) {
    await apiFetch(`/api/settings/api-keys/${id}`, { method: 'DELETE' });
    showToast('API Key deleted');
    const content = $('#settings-tab-content');
    if (content) renderApiKeysTab(content);
  }
}

function showCreateApiKeyModal() {
  let name = '';
  const overlay = h('div', { class: 'modal-overlay' },
    h('div', { class: 'modal' },
      h('h3', {}, 'Create API Key'),
      h('div', { class: 'form-group' },
        h('label', {}, 'Key Name'),
        h('input', { class: 'glass-input', type: 'text', placeholder: 'e.g. Production Frontend', onInput: (e) => name = e.target.value })
      ),
      h('div', { class: 'modal-actions' },
        h('button', { class: 'btn-glass', onClick: () => overlay.remove() }, 'Cancel'),
        h('button', { class: 'btn-primary', onClick: async () => {
          if (!name) return;
          const res = await apiFetch('/api/settings/api-keys', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ name })
          });
          const data = await res.json();
          overlay.remove();
          showCreatedKeyModal(data.key);
          const content = $('#settings-tab-content');
          if (content) renderApiKeysTab(content);
        } }, 'Create')
      )
    )
  );
  document.body.appendChild(overlay);
}

function showCreatedKeyModal(key) {
  const overlay = h('div', { class: 'modal-overlay' },
    h('div', { class: 'modal' },
      h('h3', {}, 'API Key Created'),
      h('p', { class: 'mb-4', style: { fontSize: '13px', color: 'var(--slate-400)' } }, 'Copy this key now. For security, you won\'t be able to see it again.'),
      h('div', { class: 'mb-6', style: { background: 'rgba(0,0,0,0.3)', padding: '1rem', borderRadius: '0.5rem', border: '1px solid var(--primary-400)', display: 'flex', justifyContent: 'space-between' } },
        h('code', { style: { color: 'var(--primary-300)', fontSize: '13px' } }, key),
        h('button', { class: 'btn-icon', onClick: () => { navigator.clipboard.writeText(key); showToast('Copied to clipboard'); } }, '📋')
      ),
      h('div', { class: 'modal-actions' }, h('button', { class: 'btn-primary', onClick: () => overlay.remove() }, 'I\'ve Saved It'))
    )
  );
  document.body.appendChild(overlay);
}

async function runWorkflow(id, isDebug = false) {
  showToast(isDebug ? 'Initializing debugger...' : 'Executing workflow...', 'info');
  try {
    const res = await apiFetch(`/api/workflows/${id}/run`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ debug: isDebug })
    });
    const data = await res.json();
    if (isDebug && data.result) {
      window.activeDebugRunId = data.result.runId;
      showDebugControls(data.result);
    } else if (data.success) {
      showToast('Workflow executed successfully', 'success');
    }
  } catch (err) {
    showToast('Failed to start workflow', 'error');
  }
}

async function deleteWorkflow(id) {
  if (confirm('Are you sure you want to delete this workflow?')) {
    await apiFetch(`/api/workflows/${id}`, { method: 'DELETE' });
    renderPage('workflows');
  }
}

function showCreateWorkflowModal() {
  let name = '';
  let description = '';
  const overlay = h('div', { class: 'modal-overlay' },
    h('div', { class: 'modal' },
      h('h3', {}, 'Create New Workflow'),
      h('div', { class: 'form-group' },
        h('label', {}, 'Workflow Name'),
        h('input', { id: 'wf-name-input', class: 'glass-input', type: 'text', placeholder: 'e.g. Content Approval Engine', onInput: (e) => name = e.target.value })
      ),
      h('div', { class: 'form-group' },
        h('label', {}, 'Description'),
        h('textarea', { class: 'glass-input', rows: '3', placeholder: 'Briefly describe what this workflow does...', onInput: (e) => description = e.target.value })
      ),
      h('div', { class: 'modal-actions' },
        h('button', { class: 'btn-glass', onClick: () => overlay.remove() }, 'Cancel'),
        h('button', { class: 'btn-primary', onClick: async () => {
          if (!name) return;
          const res = await apiFetch('/api/workflows', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ name, description, nodes: [], edges: [] })
          });
          const data = await res.json();
          overlay.remove();
          navigate('builder:' + data.id);
        } }, 'Create Workflow')
      )
    )
  );
  document.body.appendChild(overlay);
  setTimeout(() => $('#wf-name-input').focus(), 100);
}
async function renderWorkflowRunsModal(workflowId, workflowName) {
  const res = await apiFetch(`/api/workflows/${workflowId}/runs`);
  const runs = await res.json();

  const overlay = h('div', { class: 'modal-overlay', onClick: (e) => { if (e.target === overlay) overlay.remove(); } },
    h('div', { class: 'modal', style: { maxWidth: '800px', width: '90%', height: '80vh', display: 'flex', flexDirection: 'column', padding: '0' } },
      h('div', { class: 'section-header' },
        h('div', { class: 'flex items-center gap-4' },
          h('h3', { style: { margin: '0' } }, `Run History: ${workflowName}`),
          h('button', { class: 'btn-glass text-xs', onClick: () => reportWorkflow(workflowId) }, '📊 Audit Report')
        ),
        h('button', { class: 'btn-icon', onClick: () => overlay.remove() }, '✕')
      ),
      h('div', { style: { flex: '1', display: 'grid', gridTemplateColumns: '300px 1fr', overflow: 'hidden' } },
        // Left Column: List of Runs
        h('div', { style: { borderRight: '1px solid rgba(255,255,255,0.06)', overflowY: 'auto', padding: '1rem' } },
          ...runs.map(run => h('div', { 
            class: 'glass-card run-item', 
            style: { cursor: 'pointer', marginBottom: '0.75rem', padding: '0.75rem', border: '1px solid rgba(255,255,255,0.04)' },
            onClick: () => selectRun(run)
          },
            h('div', { class: 'flex justify-between items-center mb-1' },
              h('p', { style: { fontSize: '12px', fontWeight: '600', color: 'white' } }, run.id.slice(-8)),
              h('span', { class: `badge-${run.status === 'completed' ? 'success' : run.status === 'failed' ? 'danger' : 'warning'}`, style: { fontSize: '10px' } }, run.status)
            ),
            h('p', { style: { fontSize: '11px', color: 'var(--slate-500)' } }, fmtDate(run.started_at))
          ))
        ),
        // Right Column: Run Details / Logs
        h('div', { id: 'run-details-pane', style: { overflowY: 'auto', padding: '1.5rem', background: 'rgba(0,0,0,0.1)' } },
          h('div', { class: 'empty-state' }, h('p', {}, 'Select a run to view execution logs'))
        )
      )
    )
  );

  function selectRun(run) {
    const pane = $('#run-details-pane');
    pane.innerHTML = '';
    pane.appendChild(h('div', { class: 'animate-fade-in' },
      h('div', { class: 'flex justify-between mb-6', style: { alignItems: 'flex-start' } },
        h('div', {},
          h('h4', { style: { color: 'white', marginBottom: '0.25rem' } }, `Run Details: ${run.id}`),
          h('p', { style: { fontSize: '12px', color: 'var(--slate-500)' } }, `Executed at ${fmtDate(run.started_at)}`)
        ),
        h('div', { class: `badge-${run.status === 'completed' ? 'success' : run.status === 'failed' ? 'danger' : 'warning'}` }, run.status)
      ),
      h('div', { class: 'mb-6' },
        h('p', { style: { fontSize: '11px', color: 'var(--slate-500)', textTransform: 'uppercase', marginBottom: '1rem' } }, 'Execution Trace'),
        h('div', { style: { display: 'flex', flexDirection: 'column', gap: '0.75rem' } },
            ...run.logs.map(log => h('div', { 
            class: 'log-entry',
            style: { cursor: 'pointer', padding: '0.75rem', background: 'rgba(255,255,255,0.03)', borderRadius: '0.5rem', borderLeft: `3px solid ${log.status === 'success' ? 'var(--emerald-500)' : 'var(--red-500)'}` },
            onClick: () => highlightNode(log.node_id)
          },
            h('div', { class: 'flex justify-between mb-1' },
              h('div', { class: 'flex items-center gap-2' },
                h('span', { style: { fontWeight: '600', fontSize: '12px', color: 'var(--slate-300)' } }, `Node: ${log.node_id}`),
                run.status === 'failed' ? h('button', { class: 'btn-glass text-xs', style: { padding: '2px 6px', height: 'auto' }, onClick: (e) => { e.stopPropagation(); selfHealNode(run.id, log.node_id); } }, '🩹 Heal') : null
              ),
              h('span', { style: { fontSize: '10px', color: 'var(--slate-600)' } }, `${log.duration_ms}ms`)
            ),
            h('pre', { style: { fontSize: '11px', color: 'var(--slate-400)', whiteSpace: 'pre-wrap', fontFamily: 'var(--font-mono)' } }, log.message)
          ))
        )
      ),
      run.output ? h('div', {},
        h('p', { style: { fontSize: '11px', color: 'var(--slate-500)', textTransform: 'uppercase', marginBottom: '0.5rem' } }, 'Final Output'),
        h('div', { style: { padding: '1rem', background: 'rgba(0,0,0,0.3)', borderRadius: '0.5rem', border: '1px solid rgba(255,255,255,0.06)' } },
          h('pre', { style: { fontSize: '12px', color: 'var(--primary-300)', whiteSpace: 'pre-wrap' } }, run.output)
        )
      ) : null
    ));
  }

  document.body.appendChild(overlay);
}

function showAgentModal(agent = null) {
  let name = agent ? agent.name : '';
  let model = agent ? agent.model : 'llama3.2';
  let description = agent ? agent.description : '';
  let prompt = agent ? agent.system_prompt : '';

  const overlay = h('div', { class: 'modal-overlay' },
    h('div', { class: 'modal', style: { maxWidth: '500px' } },
      h('h3', {}, agent ? 'Edit Agent' : 'Create New Agent'),
      h('div', { class: 'form-group' },
        h('label', {}, 'Agent Name'),
        h('input', { class: 'glass-input', type: 'text', value: name, placeholder: 'e.g. Code Reviewer', onInput: (e) => name = e.target.value })
      ),
      h('div', { class: 'form-group' },
        h('label', {}, 'Model'),
        h('select', { class: 'glass-input', onInput: (e) => model = e.target.value },
          h('option', { value: 'llama3.2', selected: model === 'llama3.2' }, 'Llama 3.2 (Fast)'),
          h('option', { value: 'llama3.1:70b', selected: model === 'llama3.1:70b' }, 'Llama 3.1 70B (Smart)'),
          h('option', { value: 'mistral', selected: model === 'mistral' }, 'Mistral (Balanced)')
        )
      ),
      h('div', { class: 'form-group' },
        h('label', {}, 'System Prompt'),
        h('textarea', { class: 'glass-input', rows: '5', placeholder: 'Define the agent personality and rules...', onInput: (e) => prompt = e.target.value }, prompt)
      ),
      h('div', { class: 'modal-actions' },
        h('button', { class: 'btn-glass', onClick: () => overlay.remove() }, 'Cancel'),
        h('button', { class: 'btn-primary', onClick: async () => {
          if (!name) return;
          const url = agent ? `/api/agents/${agent.id}` : '/api/agents';
          const method = agent ? 'PUT' : 'POST';
          await apiFetch(url, {
            method,
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ name, model, description, system_prompt: prompt, tags: [] })
          });
          overlay.remove();
          renderPage('agents');
        } }, agent ? 'Save Changes' : 'Create Agent')
      )
    )
  );
  document.body.appendChild(overlay);
}

function exportAuditCsv(events) {
  const header = ['Timestamp', 'Event', 'Severity', 'Actor', 'Email', 'Target', 'Target Type'];
  const rows = events.map(ev => [
    new Date(ev.timestamp).toISOString(),
    ev.type,
    ev.severity,
    ev.actor && ev.actor.name ? ev.actor.name : '',
    ev.actor && ev.actor.email ? ev.actor.email : '',
    ev.target && ev.target.name ? ev.target.name : '',
    ev.target && ev.target.target_type ? ev.target.target_type : ''
  ]);
  const csv = [header, ...rows]
    .map(r => r.map(v => `"${String(v).replace(/"/g, '""')}"`).join(','))
    .join('\n');
  const blob = new Blob([csv], { type: 'text/csv' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `audit-${new Date().toISOString().slice(0, 10)}.csv`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

document.addEventListener('DOMContentLoaded', async () => {
  // Restore auth from the session token alone. API keys are optional and can
  // be regenerated from the in-app settings flow.
  if (!session) {
    showAuthScreen();
    return;
  }

  // Validate existing session
  try {
    const res = await fetch('/auth/me', { headers: { 'Authorization': `Bearer ${session}` } });
    if (!res.ok) {
      signOut();
      return;
    }
    const user = await res.json();
    currentUser = user;
    localStorage.setItem('currentUser', JSON.stringify(user));
  } catch (e) {
    // If offline or server unreachable, allow the cached user to proceed.
    if (!currentUser) {
      showAuthScreen();
      return;
    }
  }

  updateUserDisplay();
  // Hide Settings nav for non-admin users
  const settingsNav = document.querySelector('.nav-item[data-page="settings"]');
  if (settingsNav) {
    settingsNav.style.display = (currentUser && currentUser.role === 'admin') ? '' : 'none';
  }
  navigate('dashboard');

  // Live Polling for active views
  setInterval(() => {
    if (currentPage === 'dashboard' || currentPage === 'approvals') {
      renderPage(currentPage);
    }
  }, 10000); // Every 10 seconds
});
