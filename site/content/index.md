---
title: "MailBox Ultra: native macOS SMTP fake inbox"
description: "A native macOS app that catches every email your dev environment tries to send. Real SMTP server inside a real Mac app, HTML rendered by the system WebKit. No browser, no cloud, no real delivery."
slug: ""
layout: home
---

<section class="hero">
  <span class="hero-eyebrow"><span class="badge">v{{version}}</span> Native macOS · pure Rust + egui</span>
  <h1>Catch every email. <span class="accent">Right inside a real Mac app.</span></h1>
  <p class="lede">MailBox Ultra is a native macOS application that runs a real SMTP server in-process. Point any sender at <code>smtp://127.0.0.1:1025</code> and every message is parsed, stored, and rendered live in the app — HTML emails painted by the same WebKit engine Mail.app uses. No browser, no HTTP server, no Chromium, no cloud.</p>

  <div class="hero-actions">
    <a class="btn primary" href="{{base}}/install/">Download for macOS</a>
    <a class="btn ghost" href="{{repo}}" rel="noopener noreferrer">
      <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path fill="currentColor" fill-rule="evenodd" d="M8 0C3.58 0 0 3.58 0 8a8 8 0 0 0 5.47 7.59c.4.07.55-.17.55-.38v-1.33c-2.23.48-2.7-1.07-2.7-1.07-.36-.92-.89-1.17-.89-1.17-.73-.5.06-.49.06-.49.81.06 1.23.83 1.23.83.72 1.23 1.88.88 2.34.67.07-.52.28-.88.51-1.08-1.78-.2-3.65-.89-3.65-3.95 0-.87.31-1.59.83-2.15-.08-.21-.36-1.02.08-2.13 0 0 .67-.21 2.2.82a7.6 7.6 0 0 1 4 0c1.53-1.04 2.2-.82 2.2-.82.44 1.11.16 1.92.08 2.13.51.56.82 1.28.82 2.15 0 3.07-1.87 3.75-3.66 3.95.29.25.54.73.54 1.48v2.2c0 .21.15.46.55.38A8 8 0 0 0 16 8c0-4.42-3.58-8-8-8Z"/></svg>
      View on GitHub
    </a>
  </div>

  <div class="hero-meta">
    <span><strong>macOS 11+</strong> · universal binary, ~10 MB</span>
    <span><strong>System WebKit</strong> for HTML rendering</span>
    <span><strong>MIT</strong> licensed</span>
    <span><strong>0 telemetry</strong> · runs offline</span>
  </div>

  <figure class="hero-figure">
    <img src="{{base}}/img/screenshot.png" alt="MailBox Ultra showing a captured marketing email rendered in the desktop preview, with a sidebar of captured messages from Stripe, GitHub, Figma, and a fictional bikini brand" width="1600" height="1000" />
  </figure>
</section>

<section class="section">
  <div class="section-eyebrow">Why MailBox Ultra</div>
  <h2>The local Mac alternative to mail-staging SaaS.</h2>
  <p class="section-lede">Pointing your dev environment at a real SMTP relay is overkill, and every SaaS sandbox needs an account and an internet round-trip. MailBox Ultra is a real SMTP server inside a real Mac app — capture every message your stack sends, render the HTML pixel-perfectly inside the app window, never deliver one.</p>

  <div class="feature-grid">
    <div class="feature">
      <h3><span class="feature-icon">✉</span> Real SMTP</h3>
      <p>HELO, EHLO, MAIL FROM, RCPT TO, DATA, RSET, NOOP, QUIT, AUTH PLAIN, AUTH LOGIN. Anything that speaks RFC 5321 just works.</p>
    </div>
    <div class="feature">
      <h3><span class="feature-icon">🪟</span> Native window</h3>
      <p>Real Mac dock icon, native menu bar, <code>⌘,</code> for Preferences, <code>⌘Q</code> to quit. Window position persists across launches.</p>
    </div>
    <div class="feature">
      <h3><span class="feature-icon">📱</span> WebKit HTML preview</h3>
      <p>Captured HTML emails render via an in-app <code>WKWebView</code> — same engine Mail.app uses. Desktop / iPad / Mobile width buttons swap viewport <em>and</em> User-Agent to faithfully preview responsive emails.</p>
    </div>
    <div class="feature">
      <h3><span class="feature-icon">⇄</span> Relay mode</h3>
      <p>Optional upstream <code>smtp://</code> or <code>smtps://</code> URL. Capture for inspection, then forward to a real MTA. Toggle without restarting the SMTP listener.</p>
    </div>
    <div class="feature">
      <h3><span class="feature-icon">🔒</span> Locked-down by default</h3>
      <p>HTML preview runs with JavaScript disabled, no remote loads through nav, link clicks intercepted and shelled to your default browser. Captured email HTML is sandboxed.</p>
    </div>
    <div class="feature">
      <h3><span class="feature-icon">🗎</span> NDJSON log</h3>
      <p>Optional log file appends each captured message as one JSON object per line — never truncated. Tail it from a script or a coding agent watching alongside you.</p>
    </div>
  </div>
</section>

<section class="section">
  <div class="section-eyebrow">30-second tour</div>
  <h2>Install. Send mail. See it.</h2>
  <p class="section-lede">Drop the .app into <code>/Applications</code>, open it. The toolbar shows the SMTP URL it bound. Send anything to <code>127.0.0.1:1025</code> and it lands in the inbox in milliseconds.</p>

  <div class="tour">
    <div class="tour-step">
      <span class="step-num">01</span>
      <h3>Install</h3>
      <p>Mount the universal <code>.dmg</code>, drag <strong>MailBox Ultra.app</strong> to <code>/Applications</code>.</p>
<pre>$ open MailBoxUltra-{{version}}-universal.dmg
<span class="t-mute"># drag → /Applications</span>
<span class="t-mute"># right-click → Open on first launch</span></pre>
    </div>
    <div class="tour-step">
      <span class="step-num">02</span>
      <h3>Send mail</h3>
      <p>Anything SMTP works — Laravel, Django, Rails, Node, swaks.</p>
<pre>$ swaks --to dev@example.com --from app@example.com \
  --server <span class="t-mute">127.0.0.1:1025</span> \
  --header "Subject: Hello"</pre>
    </div>
    <div class="tour-step">
      <span class="step-num">03</span>
      <h3>Inspect it</h3>
      <p>HTML rendered by WebKit. Headers, attachments (with Save…), raw RFC 822 source, all on tab keys.</p>
<pre><span class="t-mute">📥 14:23:45</span>  app@example.com → dev@example.com  Hello  140 B</pre>
    </div>
  </div>
</section>

<section class="section">
  <div class="section-eyebrow">Built for the way you actually debug</div>
  <h2>Use cases.</h2>

  <div class="use-grid">
    <div class="use">
      <h3>Transactional mail</h3>
      <p>Password resets, invoices, signups. Every mailer in your app, no smtp config gymnastics.</p>
    </div>
    <div class="use">
      <h3>Responsive email QA</h3>
      <p>Render an HTML email pixel-perfectly. Click <strong>Mobile (390)</strong> to see the iOS Mail layout — <code>@media</code> queries fire, UA is iPhone Safari.</p>
    </div>
    <div class="use">
      <h3>Library inspection</h3>
      <p>Find out exactly what a Mailer SDK or queue worker writes on the wire — headers, MIME structure, encoding.</p>
    </div>
    <div class="use">
      <h3>Capture-and-relay</h3>
      <p>Inspect locally, then deliver upstream with one toggle. One transaction, two outcomes.</p>
    </div>
    <div class="use">
      <h3>AI-assistant pairing</h3>
      <p>NDJSON log + optional relay lets a coding agent watch live mail while you keep working in the GUI.</p>
    </div>
    <div class="use">
      <h3>Onboarding flows</h3>
      <p>Reset the buffer between test runs, replay any captured signup email from the Release tab, repeat.</p>
    </div>
  </div>
</section>

<section class="section">
  <div class="section-eyebrow">Get started</div>
  <h2>Install in 30 seconds.</h2>

  <p>Grab the universal <code>.dmg</code> from the <a href="https://github.com/MPJHorner/MailboxUltra/releases/latest">latest release</a>, drag <strong>MailBox Ultra.app</strong> into <code>/Applications</code>, and right-click → Open on first launch so Gatekeeper lets the unsigned build through.</p>

  <p>Building from source? <code>git clone</code>, then <code>make app</code> produces a Mac bundle for the host arch; <code>make app-universal</code> for both. <a href="{{base}}/install/">Full install guide →</a></p>

  <div class="cta-card">
    <div>
      <h3>Read the full reference.</h3>
      <p>Every Preferences field, every relay option, every keyboard shortcut. Searchable, mobile-friendly, always in sync with the code that ships.</p>
    </div>
    <div>
      <a class="btn primary" href="{{base}}/quick-start/">Quick start</a>
      <a class="btn ghost" href="{{base}}/configuration/">Preferences</a>
    </div>
  </div>
</section>
