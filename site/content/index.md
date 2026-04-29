---
title: "MailBox Ultra: local SMTP fake inbox"
description: "Catch every email your app tries to send on a local SMTP port. Single Rust binary, live web UI, no accounts, no cloud, no real delivery."
slug: ""
layout: home
---

<section class="hero">
  <span class="hero-eyebrow"><span class="badge">v{{version}}</span> Local-first SMTP fake inbox</span>
  <h1>Catch every email. <span class="accent">Right where you build.</span></h1>
  <p class="lede">MailBox Ultra is a local SMTP server that pretends to be your production mail relay. Point any sender at <code>localhost:1025</code> and every message is parsed, stored, and shown to you in real time. No accounts, no tunnels, no real delivery, no data leaving your laptop.</p>

  <div class="hero-actions">
    <a class="btn primary" href="{{base}}/install/">Install MailBox Ultra</a>
    <a class="btn ghost" href="{{repo}}" rel="noopener noreferrer">
      <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path fill="currentColor" fill-rule="evenodd" d="M8 0C3.58 0 0 3.58 0 8a8 8 0 0 0 5.47 7.59c.4.07.55-.17.55-.38v-1.33c-2.23.48-2.7-1.07-2.7-1.07-.36-.92-.89-1.17-.89-1.17-.73-.5.06-.49.06-.49.81.06 1.23.83 1.23.83.72 1.23 1.88.88 2.34.67.07-.52.28-.88.51-1.08-1.78-.2-3.65-.89-3.65-3.95 0-.87.31-1.59.83-2.15-.08-.21-.36-1.02.08-2.13 0 0 .67-.21 2.2.82a7.6 7.6 0 0 1 4 0c1.53-1.04 2.2-.82 2.2-.82.44 1.11.16 1.92.08 2.13.51.56.82 1.28.82 2.15 0 3.07-1.87 3.75-3.66 3.95.29.25.54.73.54 1.48v2.2c0 .21.15.46.55.38A8 8 0 0 0 16 8c0-4.42-3.58-8-8-8Z"/></svg>
      View on GitHub
    </a>
  </div>

  <div class="hero-meta">
    <span><strong>Rust</strong> · single binary, ~6 MB</span>
    <span><strong>macOS · Linux · Windows</strong></span>
    <span><strong>MIT</strong> licensed</span>
    <span><strong>0 telemetry</strong> · runs offline</span>
  </div>

  <figure class="hero-figure">
    <img src="{{base}}/img/screenshot.png" alt="MailBox Ultra web UI showing a captured email with rendered HTML body, headers, and attachments tab" width="1600" height="1000" />
  </figure>
</section>

<section class="section">
  <div class="section-eyebrow">Why MailBox Ultra</div>
  <h2>The local alternative to mail-staging SaaS.</h2>
  <p class="section-lede">Mailpit and MailHog have been around for years; they work. MailBox Ultra is the same idea rewritten in Rust, with a UI that gets out of your way, a JSON+SSE API designed for tooling, and an optional upstream relay for when you want capture *and* real delivery in the same hop.</p>

  <div class="feature-grid">
    <div class="feature">
      <h3><span class="feature-icon">⚡</span> Real-time</h3>
      <p>Live CLI stream and a Server-Sent-Events web UI. New mail appears the instant the SMTP transaction closes.</p>
    </div>
    <div class="feature">
      <h3><span class="feature-icon">✉</span> Real SMTP</h3>
      <p>HELO, EHLO, MAIL FROM, RCPT TO, DATA, RSET, NOOP, QUIT, AUTH PLAIN, AUTH LOGIN. Anything that speaks RFC 5321 just works.</p>
    </div>
    <div class="feature">
      <h3><span class="feature-icon">⇄</span> Relay mode</h3>
      <p><code>--relay</code> forwards every captured message to a real upstream MTA. Capture, then deliver — same hop.</p>
    </div>
    <div class="feature">
      <h3><span class="feature-icon">{ }</span> Smart formatters</h3>
      <p>Sandboxed HTML preview with Desktop / iPad / Mobile size switching, plain-text, header table, attachment downloads, raw RFC 822 source — every angle you'd want.</p>
    </div>
    <div class="feature">
      <h3><span class="feature-icon">▶</span> Release</h3>
      <p>Resend any captured message to a target SMTP server from the browser. Useful for replaying transactional flows.</p>
    </div>
    <div class="feature">
      <h3><span class="feature-icon">🗎</span> NDJSON log</h3>
      <p><code>--log-file</code> tails captured mail into a structured log so AI assistants can watch live traffic alongside you.</p>
    </div>
  </div>
</section>

<section class="section">
  <div class="section-eyebrow">30-second tour</div>
  <h2>Run it. Send mail. See it.</h2>
  <p class="section-lede">No flags needed. Defaults bind <code>127.0.0.1:1025</code> for SMTP and <code>127.0.0.1:8025</code> for the UI. If a port is busy, the next free one is used and the actual address is printed.</p>

  <div class="tour">
    <div class="tour-step">
      <span class="step-num">01</span>
      <h3>Run it</h3>
      <p>One binary, no config. Banner shows the URLs it bound.</p>
<pre>$ mailbox-ultra
  ✉  MailBox Ultra v{{version}}
    SMTP    <span class="t-mute">smtp://127.0.0.1:1025</span>
    Web UI  <span class="t-mute">http://127.0.0.1:8025</span></pre>
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
      <p>Terminal stream + live web UI. Sandboxed HTML preview, headers, attachments, raw source.</p>
<pre><span class="t-mute">14:23:45.123</span>  app@example.com → dev@example.com  Hello  140 B</pre>
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
      <h3>Template QA</h3>
      <p>Render an HTML email exactly the way the recipient's client would. Iterate without touching real inboxes.</p>
    </div>
    <div class="use">
      <h3>Library inspection</h3>
      <p>Find out exactly what a Mailer SDK or queue worker writes on the wire — headers, MIME structure, encoding.</p>
    </div>
    <div class="use">
      <h3>Capture-and-relay</h3>
      <p>Inspect locally, then deliver upstream with <code>--relay</code>. One transaction, two outcomes.</p>
    </div>
    <div class="use">
      <h3>AI-assistant pairing</h3>
      <p><code>--log-file</code> + <code>--relay</code> lets a coding agent watch live mail while you keep working.</p>
    </div>
    <div class="use">
      <h3>Onboarding flows</h3>
      <p>Reset the buffer between test runs, replay a captured signup email through Release, repeat.</p>
    </div>
  </div>
</section>

<section class="section">
  <div class="section-eyebrow">Get started</div>
  <h2>Install in 30 seconds.</h2>

<div class="code-block"><span class="code-lang">sh</span><button class="copy-btn" type="button" aria-label="Copy code">copy</button><pre><code class="language-sh">curl -L -o mailbox-ultra.tar.gz \
  https://github.com/MPJHorner/MailboxUltra/releases/latest/download/mailbox-ultra-aarch64-apple-darwin.tar.gz
tar -xzf mailbox-ultra.tar.gz
./mailbox-ultra</code></pre></div>

  <p>Other platforms, package managers, and source builds are listed on the <a href="{{base}}/install/">install page</a>.</p>

  <div class="cta-card">
    <div>
      <h3>Read the full reference.</h3>
      <p>Every flag, every API endpoint, every shortcut. Searchable, mobile-friendly, and always in sync with the code.</p>
    </div>
    <div>
      <a class="btn primary" href="{{base}}/cli/">CLI reference</a>
      <a class="btn ghost" href="{{base}}/api/">API reference</a>
    </div>
  </div>
</section>
