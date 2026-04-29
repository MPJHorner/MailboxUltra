#!/usr/bin/env python3
"""Fire varied SMTP messages at MailBox Ultra for previewing + dev.

Usage:
    ./scripts/simulate.py                  # run every scenario once
    ./scripts/simulate.py receipt welcome  # run named scenarios
    ./scripts/simulate.py burst -n 200     # custom burst count
    ./scripts/simulate.py --list           # list scenarios

Environment:
    SMTP_HOST    default 127.0.0.1
    SMTP_PORT    default 1025
    SMTP_AUTH    "user:pass" if AUTH is enabled in Preferences
    DELAY        seconds between sends in --all mode (default 0.25)

Stdlib only — runs on any macOS Python 3.9+ install. No pip required.
"""
from __future__ import annotations

import argparse
import base64
import os
import random
import smtplib
import struct
import sys
import time
import zlib
from datetime import datetime, timedelta, timezone
from email.message import EmailMessage
from email.utils import formataddr, formatdate, make_msgid
from typing import Callable, Dict, List, Optional

HOST = os.environ.get("SMTP_HOST", "127.0.0.1")
PORT = int(os.environ.get("SMTP_PORT", "1025"))
AUTH = os.environ.get("SMTP_AUTH")  # "user:pass" or None
DEFAULT_DELAY = float(os.environ.get("DELAY", "0.25"))


# ---------------------------------------------------------------------------
# Wire helpers
# ---------------------------------------------------------------------------


def open_session() -> smtplib.SMTP:
    s = smtplib.SMTP(HOST, PORT, timeout=10)
    s.ehlo("simulator")
    if AUTH:
        user, _, password = AUTH.partition(":")
        s.login(user, password)
    return s


def send(msg: EmailMessage, conn: Optional[smtplib.SMTP] = None) -> None:
    if "Date" not in msg:
        msg["Date"] = formatdate(localtime=True)
    if "Message-ID" not in msg:
        msg["Message-ID"] = make_msgid(domain="mailboxultra.local")
    own_conn = conn is None
    s = conn or open_session()
    try:
        s.send_message(msg)
    finally:
        if own_conn:
            s.quit()
    subject = msg["Subject"] or "(no subject)"
    print(f"  sent: {subject}", file=sys.stderr)


# ---------------------------------------------------------------------------
# Asset helpers (zero deps)
# ---------------------------------------------------------------------------


def make_png(width: int, height: int, rgb=(45, 212, 191)) -> bytes:
    """Encode a solid-colour RGB PNG from scratch with stdlib only."""
    raw = b"".join(b"\x00" + bytes(rgb) * width for _ in range(height))

    def chunk(name: bytes, data: bytes) -> bytes:
        return (
            struct.pack(">I", len(data))
            + name
            + data
            + struct.pack(">I", zlib.crc32(name + data) & 0xFFFFFFFF)
        )

    sig = b"\x89PNG\r\n\x1a\n"
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    idat = zlib.compress(raw)
    return sig + chunk(b"IHDR", ihdr) + chunk(b"IDAT", idat) + chunk(b"IEND", b"")


# A 270-byte valid PDF saying "Hello, MailBox Ultra!" in 24pt Helvetica. Easier
# to base64-embed than to assemble from scratch every call.
TINY_PDF_B64 = (
    "JVBERi0xLjQKJcOiw6PDj8OTCjEgMCBvYmo8PC9UeXBlL0NhdGFsb2cvUGFnZXMgMiAwIFI+Pg"
    "plbmRvYmoKMiAwIG9iajw8L1R5cGUvUGFnZXMvQ291bnQgMS9LaWRzWzMgMCBSXT4+CmVuZG9i"
    "agozIDAgb2JqPDwvVHlwZS9QYWdlL1BhcmVudCAyIDAgUi9NZWRpYUJveFswIDAgNDAwIDIwMF"
    "0vUmVzb3VyY2VzPDwvRm9udDw8L0YxIDQgMCBSPj4+Pi9Db250ZW50cyA1IDAgUj4+CmVuZG9i"
    "ago0IDAgb2JqPDwvVHlwZS9Gb250L1N1YnR5cGUvVHlwZTEvQmFzZUZvbnQvSGVsdmV0aWNhPj"
    "4KZW5kb2JqCjUgMCBvYmo8PC9MZW5ndGggNTI+PnN0cmVhbQpCVCAvRjEgMjQgVGYgNDAgMTQw"
    "IFRkIChIZWxsbywgTWFpbEJveCBVbHRyYSEpIFRqIEVUCmVuZHN0cmVhbQplbmRvYmoKeHJlZg"
    "owIDYKMDAwMDAwMDAwMCA2NTUzNSBmIAowMDAwMDAwMDE3IDAwMDAwIG4gCjAwMDAwMDAwNTYg"
    "MDAwMDAgbiAKMDAwMDAwMDEwOCAwMDAwMCBuIAowMDAwMDAwMjA1IDAwMDAwIG4gCjAwMDAwMD"
    "AyNjAgMDAwMDAgbiAKdHJhaWxlcjw8L1NpemUgNi9Sb290IDEgMCBSPj4Kc3RhcnR4cmVmCjM2"
    "OAolJUVPRgo="
)


def tiny_pdf() -> bytes:
    return base64.b64decode(TINY_PDF_B64)


def calendar_event(summary: str, when: datetime, duration_minutes: int = 30) -> bytes:
    end = when + timedelta(minutes=duration_minutes)
    fmt = "%Y%m%dT%H%M%SZ"
    body = (
        "BEGIN:VCALENDAR\r\n"
        "VERSION:2.0\r\n"
        "PRODID:-//MailBoxUltra//simulate//EN\r\n"
        "METHOD:REQUEST\r\n"
        "BEGIN:VEVENT\r\n"
        f"UID:{make_msgid(domain='mailboxultra.local')[1:-1]}\r\n"
        f"DTSTAMP:{datetime.now(timezone.utc).strftime(fmt)}\r\n"
        f"DTSTART:{when.astimezone(timezone.utc).strftime(fmt)}\r\n"
        f"DTEND:{end.astimezone(timezone.utc).strftime(fmt)}\r\n"
        f"SUMMARY:{summary}\r\n"
        "ORGANIZER;CN=MailBox Ultra:mailto:invite@example.com\r\n"
        "ATTENDEE;CN=Dev;RSVP=TRUE:mailto:dev@example.com\r\n"
        "DESCRIPTION:Test calendar event from simulate.py\r\n"
        "STATUS:CONFIRMED\r\n"
        "END:VEVENT\r\n"
        "END:VCALENDAR\r\n"
    )
    return body.encode("utf-8")


# ---------------------------------------------------------------------------
# Brand-y HTML template helpers
# ---------------------------------------------------------------------------


def page(content: str, accent: str = "#10b981") -> str:
    return (
        f"<!doctype html><html><body style=\"margin:0;padding:0;background:#f8fafc;"
        f"font-family:-apple-system,BlinkMacSystemFont,Inter,Segoe UI,sans-serif;color:#0f172a\">"
        f"<table width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\" style=\"background:#f8fafc\">"
        f"<tr><td align=\"center\" style=\"padding:32px 16px\">"
        f"<table width=\"560\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\" "
        f"style=\"background:#fff;border-radius:12px;overflow:hidden;box-shadow:0 1px 3px rgba(15,23,42,.08)\">"
        f"<tr><td style=\"padding:32px\">{content}</td></tr></table>"
        f"<p style=\"color:#64748b;font-size:12px;margin:16px 0 0\">"
        f"Sent by MailBox Ultra simulator · accent {accent}</p>"
        f"</td></tr></table></body></html>"
    )


def button(label: str, color: str = "#10b981") -> str:
    return (
        f"<a href=\"https://example.com\" style=\"display:inline-block;background:{color};"
        f"color:#fff;text-decoration:none;padding:12px 22px;border-radius:8px;font-weight:600;"
        f"margin:12px 0\">{label}</a>"
    )


# ---------------------------------------------------------------------------
# Scenarios
# ---------------------------------------------------------------------------


def s_plain(conn=None):
    msg = EmailMessage()
    msg["From"] = formataddr(("Alice Chen", "alice.chen@example.com"))
    msg["To"] = "dev@example.com"
    msg["Subject"] = "Lunch tomorrow?"
    msg.set_content(
        "Hey,\n\n"
        "Are we still on for lunch tomorrow at the new Vietnamese place?\n"
        "Let me know either way.\n\n"
        "— Alice\n"
    )
    send(msg, conn)


def s_welcome(conn=None):
    html = page(
        "<h1 style=\"margin:0 0 12px;font-size:26px;color:#0f172a\">Welcome to Linear 👋</h1>"
        "<p style=\"font-size:15px;line-height:1.55;color:#475569\">"
        "We're glad you're here. Linear keeps every issue, every cycle, every change visible "
        "to your whole team in one place.</p>"
        "<p>" + button("Open your workspace") + "</p>"
        "<hr style=\"border:0;border-top:1px solid #e2e8f0;margin:24px 0\"/>"
        "<p style=\"color:#475569;font-size:13px\">If you didn't sign up, you can ignore this "
        "email and we won't bug you again.</p>"
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("Linear", "team@linear.app"))
    msg["To"] = "dev@example.com"
    msg["Subject"] = "Welcome to Linear"
    msg.set_content("Welcome to Linear. Open your workspace at https://example.com")
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


def s_receipt(conn=None):
    html = page(
        "<p style=\"color:#64748b;margin:0;font-size:13px\">Order confirmation</p>"
        "<h1 style=\"margin:6px 0 4px;font-size:22px\">Order #PF-208841 confirmed</h1>"
        "<p style=\"margin:0;color:#64748b\">Estimated arrival <b>May 4 — May 6</b></p>"
        + button("Track your order")
        + "<table width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\" "
        "style=\"margin-top:24px;border-collapse:collapse\">"
        "<tr><td style=\"padding:12px 0;border-bottom:1px solid #e2e8f0\">Milky Santal candle</td>"
        "<td align=\"right\" style=\"padding:12px 0;border-bottom:1px solid #e2e8f0\"><b>$44.00</b></td></tr>"
        "<tr><td style=\"padding:12px 0;border-bottom:1px solid #e2e8f0\">Tuberose Musk incense</td>"
        "<td align=\"right\" style=\"padding:12px 0;border-bottom:1px solid #e2e8f0\"><b>$20.00</b></td></tr>"
        "<tr><td style=\"padding:12px 0;border-bottom:1px solid #e2e8f0\">Milky Santal reed diffuser</td>"
        "<td align=\"right\" style=\"padding:12px 0;border-bottom:1px solid #e2e8f0\"><b>$40.00</b></td></tr>"
        "<tr><td style=\"padding:12px 0;color:#64748b\">Subtotal</td>"
        "<td align=\"right\" style=\"padding:12px 0;color:#64748b\">$104.00</td></tr>"
        "<tr><td style=\"padding:12px 0;color:#64748b\">Shipping</td>"
        "<td align=\"right\" style=\"padding:12px 0;color:#64748b\">$0.00</td></tr>"
        "<tr><td style=\"padding:12px 0;font-size:18px\"><b>Total</b></td>"
        "<td align=\"right\" style=\"padding:12px 0;font-size:18px\"><b>$104.00</b></td></tr>"
        "</table>"
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("Portland Co.", "orders@portlandco.com"))
    msg["To"] = formataddr(("Samira Aslan", "samira@example.com"))
    msg["Subject"] = "Order #PF-208841 confirmed — thanks, Samira"
    msg.set_content(
        "Order #PF-208841 confirmed. Estimated arrival May 4 — May 6.\n"
        "Total $104.00. Track at https://example.com\n"
    )
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


def s_shipping(conn=None):
    html = page(
        "<h1 style=\"margin:0 0 8px;font-size:24px\">📦 Your order is on its way</h1>"
        "<p style=\"color:#475569\">Tracking number <code style=\"background:#f1f5f9;"
        "padding:2px 6px;border-radius:4px\">ZX9281827</code> · Carrier: UPS</p>"
        + button("Track package", "#0ea5e9")
        + "<p style=\"color:#64748b;font-size:13px;margin-top:24px\">Expected delivery: "
        "<b style=\"color:#0f172a\">Monday, May 5</b> by 8:00pm.</p>"
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("Portland Co.", "shipping@portlandco.com"))
    msg["To"] = "samira@example.com"
    msg["Subject"] = "📦 Your Portland Co. order has shipped"
    msg.set_content("Your order shipped. Tracking ZX9281827.")
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


def s_password_reset(conn=None):
    code = f"{random.randint(100000, 999999)}"
    html = page(
        "<h1 style=\"margin:0 0 12px;font-size:24px\">Reset your password</h1>"
        "<p style=\"color:#475569\">Use the code below to reset your password. "
        "It expires in 30 minutes.</p>"
        f"<div style=\"font-size:34px;font-weight:700;letter-spacing:8px;text-align:center;"
        f"background:#f1f5f9;border:1px solid #e2e8f0;padding:18px;border-radius:8px;"
        f"font-family:ui-monospace,SF Mono,Menlo,monospace;color:#0f172a\">{code}</div>"
        "<p style=\"color:#64748b;font-size:13px;margin-top:24px\">Didn't request this? "
        "You can safely ignore this email.</p>"
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("Stripe", "noreply@stripe.com"))
    msg["To"] = "dev@example.com"
    msg["Subject"] = f"Your Stripe verification code: {code}"
    msg.set_content(
        f"Your Stripe verification code is {code}. It expires in 30 minutes.\n"
        "If you didn't request this, ignore this email.\n"
    )
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


def s_newsletter(conn=None):
    html = page(
        "<p style=\"color:#10b981;font-weight:600;font-size:13px;letter-spacing:.04em;"
        "text-transform:uppercase;margin:0\">Linear newsletter · April 2026</p>"
        "<h1 style=\"margin:6px 0 16px;font-size:28px;line-height:1.2\">A faster cycle planner, "
        "smarter Slack hand-offs, and the new Asks UI</h1>"
        "<p style=\"color:#475569;line-height:1.6\">Hey there 👋 — here's what shipped this month.</p>"
        "<h2 style=\"font-size:18px;margin-top:28px\">Cycle planner, rebuilt</h2>"
        "<p style=\"color:#475569;line-height:1.6\">The cycle planner now drafts the next two "
        "cycles for you based on actual capacity, not gut feel. Drag to rebalance, click to "
        "explain why an issue jumped.</p>"
        "<h2 style=\"font-size:18px;margin-top:24px\">Slack hand-offs that don't drop context</h2>"
        "<p style=\"color:#475569;line-height:1.6\">When an issue gets reassigned, the new "
        "owner gets a Slack thread with the full discussion summary, not just the bare link.</p>"
        "<h2 style=\"font-size:18px;margin-top:24px\">Asks · public preview</h2>"
        "<p style=\"color:#475569;line-height:1.6\">Anyone in your org can file an Ask without "
        "knowing how Linear is structured. Triage routes them.</p>"
        + button("Read the full release", "#6366f1")
        + "<hr style=\"border:0;border-top:1px solid #e2e8f0;margin:32px 0\"/>"
        "<p style=\"color:#94a3b8;font-size:12px;text-align:center\">"
        "You're receiving this because you signed up at linear.app. "
        "<a href=\"#\" style=\"color:#94a3b8\">Unsubscribe</a> · "
        "<a href=\"#\" style=\"color:#94a3b8\">Manage preferences</a></p>"
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("Linear", "newsletter@linear.app"))
    msg["To"] = "dev@example.com"
    msg["Subject"] = "April: a smarter cycle planner, Slack hand-offs, Asks preview"
    msg["List-Unsubscribe"] = "<https://example.com/unsubscribe>"
    msg.set_content("Linear monthly newsletter — read at https://example.com")
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


def s_sale_alert(conn=None):
    html = page(
        "<div style=\"background:linear-gradient(135deg,#dc2626 0%,#ea580c 100%);color:#fff;"
        "padding:48px 24px;text-align:center;border-radius:8px\">"
        "<p style=\"margin:0;font-size:14px;letter-spacing:.06em\">FLASH SALE — 36 HOURS ONLY</p>"
        "<h1 style=\"font-size:54px;margin:8px 0 4px;line-height:1\">50% OFF</h1>"
        "<p style=\"margin:0;font-size:18px\">storewide · ends Sunday midnight</p></div>"
        "<p style=\"text-align:center\">" + button("Shop the sale", "#dc2626") + "</p>"
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("Glossier", "marketing@glossier.com"))
    msg["To"] = "dev@example.com"
    msg["Subject"] = "🔥 50% off storewide — 36 hours only"
    msg.set_content("Flash sale: 50% off storewide. Ends Sunday midnight.")
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


def s_github_notification(conn=None):
    body = (
        "@samira commented on this issue:\n\n"
        "> The HTML preview iframe gets stuck on the placeholder when you switch messages quickly.\n"
        "> Reproducible on Safari 17.4.\n\n"
        "Fixed in #98 — landed on main this morning.\n"
        "—\n"
        "Reply to this email directly, or view it on GitHub:\n"
        "https://github.com/MPJHorner/MailboxUltra/issues/42\n"
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("samira (via GitHub)", "notifications@github.com"))
    msg["To"] = "matt@example.com"
    msg["Subject"] = "Re: [MPJHorner/MailboxUltra] HTML preview gets stuck (#42)"
    msg["In-Reply-To"] = "<MPJHorner/MailboxUltra/issues/42@github.com>"
    msg["References"] = "<MPJHorner/MailboxUltra/issues/42@github.com>"
    msg["List-ID"] = "MPJHorner/MailboxUltra <MailboxUltra.MPJHorner.github.com>"
    msg["X-GitHub-Sender"] = "samira"
    msg.set_content(body)
    send(msg, conn)


def s_ci_failure(conn=None):
    html = page(
        "<h1 style=\"margin:0 0 8px;font-size:22px;color:#dc2626\">❌ Build #1247 failed</h1>"
        "<p style=\"color:#64748b;margin:0\">main · pushed by matt · 1m 47s</p>"
        "<pre style=\"background:#0f172a;color:#fca5a5;padding:14px 16px;border-radius:8px;"
        "font-size:12.5px;overflow:auto;margin-top:16px\">"
        "error[E0277]: the trait `Send` is not implemented for `Rc&lt;RefCell&lt;State&gt;&gt;`\n"
        "  --&gt; src/server.rs:138:9\n"
        "    |\n"
        "138 |         tokio::spawn(async move {\n"
        "    |         ^^^^^^^^^^^^ within `[closure@src/server.rs:138:22]`\n"
        "</pre>"
        + button("View workflow run", "#0ea5e9")
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("GitHub Actions", "noreply@github.com"))
    msg["To"] = "matt@example.com"
    msg["Subject"] = "[MailboxUltra] Build failed on main"
    msg.set_content("Build #1247 failed on main.")
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


def s_monitor_alert(conn=None):
    html = page(
        "<div style=\"background:#fee2e2;border:1px solid #fecaca;border-radius:8px;padding:16px\">"
        "<p style=\"margin:0;color:#dc2626;font-weight:600;font-size:14px\">🚨 Triggered</p>"
        "<h1 style=\"margin:6px 0 4px;font-size:20px\">API latency p99 &gt; 500ms (warn)</h1>"
        "<p style=\"margin:0;color:#7f1d1d\">api.example.com · region us-east-1 · 3m</p></div>"
        "<table width=\"100%\" style=\"margin-top:20px;border-collapse:collapse\">"
        "<tr><td style=\"padding:8px 0;color:#64748b\">p50</td><td align=\"right\">82 ms</td></tr>"
        "<tr><td style=\"padding:8px 0;color:#64748b\">p95</td><td align=\"right\">312 ms</td></tr>"
        "<tr><td style=\"padding:8px 0;color:#64748b\">p99</td><td align=\"right\"><b style=\"color:#dc2626\">748 ms</b></td></tr>"
        "</table>"
        + button("View dashboard", "#dc2626")
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("Datadog", "alerts@datadoghq.com"))
    msg["To"] = "oncall@example.com"
    msg["Subject"] = "[Triggered] API latency p99 > 500ms (warn)"
    msg["X-Datadog-Event-Type"] = "alert"
    msg.set_content("p99 API latency hit 748ms. View dashboard at https://example.com")
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


def s_survey(conn=None):
    cells = "".join(
        f"<td align=\"center\" style=\"padding:8px 4px\">"
        f"<a href=\"#\" style=\"display:inline-block;width:36px;height:36px;line-height:36px;"
        f"border-radius:50%;background:#f1f5f9;color:#0f172a;text-decoration:none;font-weight:600\">"
        f"{n}</a></td>"
        for n in range(0, 11)
    )
    html = page(
        "<h1 style=\"margin:0 0 12px\">How likely are you to recommend MailBox Ultra to a friend?</h1>"
        f"<table width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" style=\"margin:16px 0\"><tr>{cells}</tr></table>"
        "<p style=\"display:flex;justify-content:space-between;color:#94a3b8;font-size:12px\">"
        "<span>Not at all</span><span>Extremely likely</span></p>"
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("MailBox Ultra", "feedback@mailboxultra.local"))
    msg["To"] = "dev@example.com"
    msg["Subject"] = "Quick question: how was your visit?"
    msg.set_content("Rate us 0-10. https://example.com/survey")
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


def s_calendar_invite(conn=None):
    when = datetime.now(timezone.utc).replace(microsecond=0) + timedelta(days=1)
    ics = calendar_event("Team standup — Wednesday", when, duration_minutes=30)
    html = page(
        "<p style=\"color:#64748b;margin:0;font-size:13px\">Calendar invite</p>"
        "<h1 style=\"margin:6px 0 4px;font-size:22px\">Team standup — Wednesday</h1>"
        f"<p style=\"margin:0;color:#475569\"><b>{when:%A, %B %-d}</b> · "
        f"{when:%H:%M}–{(when + timedelta(minutes=30)):%H:%M} UTC</p>"
        + button("Open in Calendar", "#7c3aed")
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("Google Calendar", "calendar-noreply@google.com"))
    msg["To"] = "dev@example.com"
    msg["Subject"] = "Invitation: Team standup — Wednesday @ Wed Apr 30, 2026 14:00 UTC"
    msg.set_content("Team standup — Wednesday. Add to calendar: see attached .ics")
    msg.add_alternative(html, subtype="html")
    msg.add_attachment(
        ics,
        maintype="text",
        subtype="calendar",
        filename="invite.ics",
    )
    send(msg, conn)


def s_with_pdf(conn=None):
    html = page(
        "<h1 style=\"margin:0 0 8px;font-size:22px\">Invoice #INV-00482</h1>"
        "<p style=\"color:#475569\">Attached is your invoice for April. "
        "Total due: <b>$1,240.00</b> by May 15.</p>"
        + button("Pay invoice")
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("Acme Billing", "billing@acme.example"))
    msg["To"] = "ap@example.com"
    msg["Subject"] = "Invoice #INV-00482 — $1,240.00 due May 15"
    msg.set_content("Invoice attached. Total $1,240.00 due May 15.")
    msg.add_alternative(html, subtype="html")
    msg.add_attachment(
        tiny_pdf(),
        maintype="application",
        subtype="pdf",
        filename="INV-00482.pdf",
    )
    send(msg, conn)


def s_with_image(conn=None):
    html = page(
        "<h1 style=\"margin:0 0 8px\">Your weekly snapshot</h1>"
        "<p style=\"color:#475569\">Here's how your team did this week. "
        "Chart attached for the trend.</p>"
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("Stats", "stats@example.com"))
    msg["To"] = "dev@example.com"
    msg["Subject"] = "Weekly snapshot 📊"
    msg.set_content("See attached chart.")
    msg.add_alternative(html, subtype="html")
    msg.add_attachment(
        make_png(64, 64, rgb=(45, 212, 191)),
        maintype="image",
        subtype="png",
        filename="snapshot.png",
    )
    msg.add_attachment(
        make_png(96, 32, rgb=(99, 102, 241)),
        maintype="image",
        subtype="png",
        filename="trend.png",
    )
    send(msg, conn)


def s_text_with_attachment(conn=None):
    msg = EmailMessage()
    msg["From"] = "logs@example.com"
    msg["To"] = "dev@example.com"
    msg["Subject"] = "Daily log digest"
    msg.set_content(
        "Daily log digest — 12 entries.\nSee attached digest.txt for details.\n"
    )
    digest = (
        "2026-04-29 09:00:01  INFO   server bound 127.0.0.1:1025\n"
        "2026-04-29 09:00:01  INFO   web ui bound 127.0.0.1:8025\n"
        "2026-04-29 09:14:22  WARN   port 1025 in use, fell back to 1026\n"
        "2026-04-29 10:02:14  INFO   message captured: <abc@x>\n"
    )
    msg.add_attachment(
        digest.encode("utf-8"),
        maintype="text",
        subtype="plain",
        filename="digest.txt",
    )
    send(msg, conn)


def s_many_recipients(conn=None):
    msg = EmailMessage()
    msg["From"] = formataddr(("Project Manager", "pm@example.com"))
    msg["To"] = ", ".join(
        formataddr((n, f"{n.lower()}@example.com"))
        for n in ["Alice", "Bob"]
    )
    msg["Cc"] = ", ".join(
        formataddr((n, f"{n.lower()}@example.com"))
        for n in ["Carol", "Dan", "Eve", "Frank", "Grace"]
    )
    msg["Subject"] = "Sprint review — Friday at 3pm (please confirm)"
    msg.set_content(
        "Hi all — sprint review this Friday at 3pm. Please reply with a thumbs-up "
        "if you can make it."
    )
    send(msg, conn)


def s_unicode(conn=None):
    body = (
        "Hello from around the world — here's a smattering of writing systems:\n\n"
        "  English:  Hello, world.\n"
        "  Café:     café crème, naïve résumé\n"
        "  العربية:  مرحبا بالعالم\n"
        "  עברית:    שלום עולם\n"
        "  中文:     你好,世界\n"
        "  日本語:   こんにちは、世界\n"
        "  한국어:   안녕하세요, 세계\n"
        "  Emoji:    🚀✉️🎉☕✨\n"
        "  Math:     ∑∇∂√π∞≠≈⊆\n"
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("Mō Sakura", "mo.sakura@example.com"))
    msg["To"] = "dev@example.com"
    msg["Subject"] = "🚀 Café · 北京 · שלום · résumé"
    msg.set_content(body)
    send(msg, conn)


def s_encoded_subject(conn=None):
    # Force the subject to round-trip as RFC 2047 even though it's also valid
    # UTF-8 — useful for testing the parser.
    msg = EmailMessage()
    msg["From"] = "test@example.com"
    msg["To"] = "dev@example.com"
    msg[
        "Subject"
    ] = "=?utf-8?B?8J+TpiBZb3VyIE1haWxCb3ggVWx0cmEgcGFja2FnZSBpcyBoZXJlIQ==?="
    msg.set_content("Encoded-word subject — should still render '📦 Your MailBox Ultra package is here!'")
    send(msg, conn)


def s_long_subject(conn=None):
    long = (
        "[NOTIFY] Your monthly subscription to the Premium tier of the MailBox Ultra "
        "developer-tools observability suite has been processed successfully and an "
        "invoice receipt has been generated and is attached to this email for your records"
    )
    msg = EmailMessage()
    msg["From"] = "billing@example.com"
    msg["To"] = "dev@example.com"
    msg["Subject"] = long
    msg.set_content("Long subject ahead. Body is intentionally short.")
    send(msg, conn)


def s_long_body(conn=None):
    paragraph = (
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit. "
        "Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. "
        "Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris "
        "nisi ut aliquip ex ea commodo consequat. "
        "Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore "
        "eu fugiat nulla pariatur.\n\n"
    )
    msg = EmailMessage()
    msg["From"] = "novella@example.com"
    msg["To"] = "dev@example.com"
    msg["Subject"] = "A novella in your inbox"
    msg.set_content(paragraph * 80)
    send(msg, conn)


def s_no_subject(conn=None):
    msg = EmailMessage()
    msg["From"] = "ghost@example.com"
    msg["To"] = "dev@example.com"
    msg.set_content("This message intentionally has no Subject header.")
    send(msg, conn)


def s_html_only(conn=None):
    html = page(
        "<h1>HTML-only message</h1>"
        "<p>There is no <code>text/plain</code> alternative. Mail.app would prefer "
        "the HTML; the simulator wants to see how the app handles a single-part HTML "
        "body without a sibling.</p>"
    )
    msg = EmailMessage()
    msg["From"] = "html-only@example.com"
    msg["To"] = "dev@example.com"
    msg["Subject"] = "HTML-only test (no text/plain fallback)"
    msg.set_content(html, subtype="html")
    send(msg, conn)


def s_reply_thread(conn=None):
    parent_id = make_msgid(domain="mailboxultra.local")
    grand_id = make_msgid(domain="mailboxultra.local")
    body = (
        "Confirmed for Friday — see you at 3.\n\n"
        "On Tue, Apr 28 at 4:12 PM, Bob wrote:\n"
        "> Friday at 3 works on my end. Want me to grab the room?\n\n"
        ">> On Tue, Apr 28 at 3:48 PM, Alice wrote:\n"
        ">> Can we do Friday afternoon instead?\n"
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("Alice", "alice@example.com"))
    msg["To"] = formataddr(("Bob", "bob@example.com"))
    msg["Subject"] = "Re: Re: Sprint review timing"
    msg["In-Reply-To"] = parent_id
    msg["References"] = f"{grand_id} {parent_id}"
    msg.set_content(body)
    send(msg, conn)


def s_dark_mode_aware(conn=None):
    html = (
        "<!doctype html><html><head>"
        "<meta name=\"color-scheme\" content=\"light dark\"/>"
        "<meta name=\"supported-color-schemes\" content=\"light dark\"/>"
        "<style>"
        ":root{color-scheme:light dark;}"
        "body{background:#ffffff;color:#0f172a;font-family:system-ui;margin:0;padding:32px}"
        "@media (prefers-color-scheme: dark){"
        "body{background:#0f172a;color:#e2e8f0}"
        ".card{background:#1e293b!important;border-color:#334155!important}"
        "}"
        ".card{background:#f8fafc;border:1px solid #e2e8f0;border-radius:8px;padding:18px}"
        "</style></head><body>"
        "<h1>Dark-mode aware email</h1>"
        "<div class=\"card\">"
        "<p>This email reads its colour scheme from the rendering engine. "
        "Toggle the app's theme to see the email follow.</p></div>"
        "</body></html>"
    )
    msg = EmailMessage()
    msg["From"] = "dark@example.com"
    msg["To"] = "dev@example.com"
    msg["Subject"] = "Dark-mode aware HTML"
    msg.set_content("Toggle the app theme to see this email respond.")
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


def s_marketing_image_heavy(conn=None):
    html = page(
        "<table width=\"100%\" cellpadding=\"0\" cellspacing=\"0\"><tr><td>"
        "<h1 style=\"margin:0 0 8px;font-size:34px;line-height:1.05\">"
        "<span style=\"background:linear-gradient(90deg,#10b981,#0ea5e9);"
        "-webkit-background-clip:text;background-clip:text;color:transparent\">Spring</span> "
        "is in the air</h1>"
        "<p style=\"color:#475569;font-size:16px\">Pick three new pieces from our March drop.</p>"
        "</td></tr></table>"
        "<table width=\"100%\" style=\"margin-top:24px;border-collapse:separate;border-spacing:8px\">"
        "<tr>"
        "<td width=\"33%\" style=\"background:#fef3c7;border-radius:10px;padding:16px;text-align:center\">"
        "<div style=\"font-size:32px\">🌷</div><b>Tulip linen</b><br/><span style=\"color:#92400e\">$58</span></td>"
        "<td width=\"33%\" style=\"background:#dcfce7;border-radius:10px;padding:16px;text-align:center\">"
        "<div style=\"font-size:32px\">🌿</div><b>Olive denim</b><br/><span style=\"color:#166534\">$84</span></td>"
        "<td width=\"33%\" style=\"background:#dbeafe;border-radius:10px;padding:16px;text-align:center\">"
        "<div style=\"font-size:32px\">🌊</div><b>Wave gauze</b><br/><span style=\"color:#1d4ed8\">$72</span></td>"
        "</tr></table>"
        + button("Shop the drop")
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("Field & Co", "shop@fieldandco.example"))
    msg["To"] = "dev@example.com"
    msg["Subject"] = "🌷 Spring drop — three new pieces"
    msg.set_content("Spring drop is live. Shop at https://example.com")
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


# ---------------------------------------------------------------------------
# MARÉ — fictional swim brand. Email-safe responsive HTML, real Unsplash
# product/lifestyle imagery, brand wordmark via inline CSS so it always
# renders even when the WKWebView blocks remote loads. Designed as a
# "perfectly curated" reference for previewing the HTML pane.
# ---------------------------------------------------------------------------


# Stable Unsplash CDN URLs — high-quality bikini / swim / beach imagery.
# Append `?w=NNN&auto=format&fit=crop&q=80` for sized variants.
MARE_HERO = (
    "https://images.unsplash.com/photo-1605248259586-a64eb06b6970"
    "?w=1200&auto=format&fit=crop&q=85"
)
MARE_HERO_2 = (
    "https://images.unsplash.com/photo-1574539602047-548bf9557352"
    "?w=1200&auto=format&fit=crop&q=85"
)
MARE_HERO_BEACH = (
    "https://images.unsplash.com/photo-1535262412227-85541e910204"
    "?w=1200&auto=format&fit=crop&q=85"
)
MARE_PRODUCT = [
    # (image, name, price, color)
    (
        "https://images.unsplash.com/photo-1531469535976-c6fc3604014f"
        "?w=600&auto=format&fit=crop&q=85",
        "Lina Triangle Top",
        "$98",
        "Sandstone",
    ),
    (
        "https://images.unsplash.com/photo-1568819317551-31051b37f69f"
        "?w=600&auto=format&fit=crop&q=85",
        "Olive Tie Bottom",
        "$84",
        "Olive",
    ),
    (
        "https://images.unsplash.com/photo-1581588636584-5c447d2c9d97"
        "?w=600&auto=format&fit=crop&q=85",
        "Marina One-Piece",
        "$148",
        "Black",
    ),
    (
        "https://images.unsplash.com/photo-1611145434336-2324aa4079cd"
        "?w=600&auto=format&fit=crop&q=85",
        "Sol Ruffle Top",
        "$92",
        "Coral",
    ),
    (
        "https://images.unsplash.com/photo-1623039497026-00af61471107"
        "?w=600&auto=format&fit=crop&q=85",
        "Reef Cheeky Bottom",
        "$78",
        "Ivory",
    ),
    (
        "https://images.unsplash.com/photo-1467632499275-7a693a761056"
        "?w=600&auto=format&fit=crop&q=85",
        "Cala Wrap Skirt",
        "$112",
        "Driftwood",
    ),
]


def _mare_shell(preheader: str, body: str, footer_note: str = "") -> str:
    """600px responsive table-based shell with the MARÉ wordmark header.

    Uses inline styles for everything that needs to render in Gmail (which
    strips <style> blocks aggressively), plus a single <style> block with
    `@media (max-width:540px)` rules that Apple Mail / iOS Mail / Outlook 2019+
    honour to stack columns and shrink padding on phones. WKWebView (our
    preview) follows the @media rules too, so the Mobile (390px) device
    button gets a real mobile layout — not just a squashed desktop one.

    The wordmark is rendered as styled text so the header always reads even
    if image loading is blocked. The hidden preheader controls inbox preview.
    """
    note = footer_note or (
        "You're receiving this because you signed up at maré.swim. "
        "Manage preferences or unsubscribe."
    )
    return f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width,initial-scale=1"/>
  <meta name="color-scheme" content="light"/>
  <style>
    /* Honoured by Apple Mail / iOS Mail / Outlook 2019+ / WKWebView preview.
       Gmail strips <style>, but the inline desktop styles still hold there
       and the layout shrinks gracefully via width:100% on images. */
    @media only screen and (max-width:540px) {{
      .mare-shell {{ width:100% !important; max-width:100% !important; }}
      .mare-stack {{ display:block !important; width:100% !important;
                     padding:0 0 16px 0 !important; box-sizing:border-box; }}
      .mare-pad-lg {{ padding-left:20px !important; padding-right:20px !important; }}
      .mare-pad-md {{ padding-left:16px !important; padding-right:16px !important; }}
      .mare-pad-section {{ padding:28px 20px 0 20px !important; }}
      .mare-pad-section-tight {{ padding:24px 20px 0 20px !important; }}
      .mare-pad-bottom {{ padding-bottom:32px !important; }}
      .mare-hero {{ font-size:28px !important; line-height:1.15 !important; }}
      .mare-hero-xl {{ font-size:38px !important; }}
      .mare-eyebrow {{ letter-spacing:3px !important; }}
      .mare-banner-text {{ letter-spacing:3px !important; font-size:10px !important; }}
      .mare-card-img {{ max-width:100% !important; }}
    }}
    /* Reset some Outlook/Hotmail quirks. */
    img {{ -ms-interpolation-mode:bicubic; }}
    table {{ border-collapse:collapse; mso-table-lspace:0; mso-table-rspace:0; }}
    a {{ text-decoration:none; }}
  </style>
</head>
<body style="margin:0;padding:0;background:#f6f1ea;font-family:'Helvetica Neue',Helvetica,Arial,sans-serif;color:#1a1a1a;-webkit-font-smoothing:antialiased">
  <div style="display:none;max-height:0;overflow:hidden;opacity:0">{preheader}</div>
  <table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" style="background:#f6f1ea">
    <tr><td align="center" style="padding:24px 12px">
      <table role="presentation" class="mare-shell" width="600" cellpadding="0" cellspacing="0" border="0" style="background:#ffffff;border-radius:2px;max-width:600px;width:100%">
        <tr><td align="center" style="padding:28px 24px 20px 24px;border-bottom:1px solid #ece6dd">
          <span style="display:inline-block;font-family:'Helvetica Neue',Helvetica,Arial,sans-serif;font-size:22px;font-weight:300;letter-spacing:14px;color:#1a1a1a;text-transform:uppercase">MAR&Eacute;</span>
          <div class="mare-eyebrow" style="font-size:9px;letter-spacing:3px;color:#9c8d7a;margin-top:6px;text-transform:uppercase">Swim · Resort · Sun</div>
        </td></tr>
        <tr><td>{body}</td></tr>
        <tr><td class="mare-pad-md" style="padding:28px 28px 24px 28px;border-top:1px solid #ece6dd;background:#faf7f2">
          <table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0">
            <tr>
              <td align="center" style="padding-bottom:14px">
                <a href="https://example.com/instagram" style="text-decoration:none;color:#9c8d7a;font-size:11px;letter-spacing:2px;margin:0 10px;text-transform:uppercase">Instagram</a>
                <a href="https://example.com/tiktok" style="text-decoration:none;color:#9c8d7a;font-size:11px;letter-spacing:2px;margin:0 10px;text-transform:uppercase">TikTok</a>
                <a href="https://example.com/help" style="text-decoration:none;color:#9c8d7a;font-size:11px;letter-spacing:2px;margin:0 10px;text-transform:uppercase">Help</a>
              </td>
            </tr>
            <tr><td align="center" style="font-size:11px;line-height:1.6;color:#9c8d7a">
              {note}<br/>
              MAR&Eacute; Swim Co · 121 Ocean Avenue · Santa Monica CA 90402
            </td></tr>
          </table>
        </td></tr>
      </table>
    </td></tr>
  </table>
</body>
</html>"""


def _mare_button(label: str, href: str = "https://example.com") -> str:
    """Bulletproof button — table-wrapped so Outlook renders it correctly."""
    return (
        f'<table role="presentation" cellpadding="0" cellspacing="0" border="0" '
        f'style="margin:0 auto"><tr><td align="center" bgcolor="#1a1a1a" '
        f'style="border-radius:0;background:#1a1a1a">'
        f'<a href="{href}" style="display:inline-block;padding:14px 36px;'
        f'font-family:Helvetica,Arial,sans-serif;font-size:12px;'
        f'letter-spacing:3px;color:#ffffff;text-decoration:none;'
        f'text-transform:uppercase;font-weight:500">{label}</a>'
        f'</td></tr></table>'
    )


def _mare_product_card(img: str, name: str, price: str, color: str) -> str:
    """Single product cell — 100% width within its parent column.

    The image uses `width:100%; max-width:520px` so it scales up on a
    full-width mobile column (where its parent td is now 100%, not 50%).
    """
    return (
        f'<table role="presentation" width="100%" cellpadding="0" cellspacing="0" '
        f'border="0"><tr><td>'
        f'<a href="https://example.com" style="text-decoration:none;color:#1a1a1a">'
        f'<img src="{img}" width="260" alt="{name}" class="mare-card-img" '
        f'style="display:block;width:100%;max-width:520px;height:auto;border:0"/>'
        f'<div style="padding:12px 4px 0 4px">'
        f'<div style="font-size:13px;letter-spacing:1px;color:#1a1a1a;'
        f'text-transform:uppercase">{name}</div>'
        f'<div style="font-size:11px;letter-spacing:1.5px;color:#9c8d7a;'
        f'margin-top:3px;text-transform:uppercase">{color}</div>'
        f'<div style="font-size:13px;color:#1a1a1a;margin-top:6px">{price}</div>'
        f'</div></a></td></tr></table>'
    )


def s_mare_welcome(conn=None):
    """Welcome email — hero photo, brand intro, single CTA."""
    body = f"""
      <tr><td>
        <img src="{MARE_HERO}" width="600" alt="MARÉ Swim — Resort '26 collection"
             style="display:block;width:100%;height:auto;border:0"/>
      </td></tr>
      <tr><td class="mare-pad-md" style="padding:40px 48px 16px 48px;text-align:center">
        <div class="mare-eyebrow" style="font-size:11px;letter-spacing:4px;color:#9c8d7a;text-transform:uppercase">Welcome</div>
        <h1 class="mare-hero" style="font-family:Georgia,'Times New Roman',serif;font-weight:300;font-size:36px;line-height:1.15;margin:14px 0 0 0;color:#1a1a1a">
          The shore<br/>is calling.
        </h1>
        <p style="font-size:15px;line-height:1.7;color:#52483b;margin:18px 16px 28px 16px">
          Welcome to MARÉ. Considered swim made in small batches from
          recycled-nylon Carvico Vita and trimmed with handcrafted shell beads.
          Use code <b style="letter-spacing:1px">FIRSTSHORE</b> for 15% off your first piece.
        </p>
      </td></tr>
      <tr><td align="center" class="mare-pad-md" style="padding:0 48px 48px 48px">
        {_mare_button("Shop the new collection")}
      </td></tr>
    """
    html = _mare_shell("Welcome to MARÉ — 15% off your first piece", body)
    text = (
        "Welcome to MARÉ.\n\n"
        "Considered swim made in small batches from recycled-nylon Carvico\n"
        "Vita. Use code FIRSTSHORE for 15% off your first piece.\n\n"
        "Shop the new collection: https://example.com\n"
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("MARÉ", "hello@mare.swim"))
    msg["To"] = formataddr(("Julia Park", "julia@example.com"))
    msg["Subject"] = "Welcome to MARÉ — your first 15% is on us"
    msg.set_content(text)
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


def s_mare_drop(conn=None):
    """New collection drop — hero, 4-product grid, CTA."""
    p = MARE_PRODUCT
    body = f"""
      <tr><td>
        <img src="{MARE_HERO_2}" width="600" alt="Resort '26 — six new pieces"
             style="display:block;width:100%;height:auto;border:0"/>
      </td></tr>
      <tr><td class="mare-pad-md" style="padding:36px 48px 0 48px;text-align:center">
        <div class="mare-eyebrow" style="font-size:11px;letter-spacing:4px;color:#9c8d7a;text-transform:uppercase">Just landed</div>
        <h1 class="mare-hero" style="font-family:Georgia,'Times New Roman',serif;font-weight:300;font-size:32px;line-height:1.15;margin:12px 0 4px 0;color:#1a1a1a">
          Resort &rsquo;26
        </h1>
        <p style="font-size:14px;line-height:1.7;color:#52483b;margin:8px 16px 32px 16px">
          Six new pieces in colours pulled from the Pacific coastline at dusk.
        </p>
      </td></tr>
      <tr><td class="mare-pad-md" style="padding:0 48px 8px 48px">
        <table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0">
          <tr>
            <td class="mare-stack" width="50%" valign="top" style="padding:0 8px 16px 0">{_mare_product_card(*p[0])}</td>
            <td class="mare-stack" width="50%" valign="top" style="padding:0 0 16px 8px">{_mare_product_card(*p[1])}</td>
          </tr>
          <tr>
            <td class="mare-stack" width="50%" valign="top" style="padding:0 8px 16px 0">{_mare_product_card(*p[2])}</td>
            <td class="mare-stack" width="50%" valign="top" style="padding:0 0 16px 8px">{_mare_product_card(*p[3])}</td>
          </tr>
        </table>
      </td></tr>
      <tr><td align="center" class="mare-pad-md" style="padding:24px 48px 48px 48px">
        {_mare_button("Shop the drop")}
      </td></tr>
    """
    html = _mare_shell("Resort '26 has landed — six new pieces", body)
    msg = EmailMessage()
    msg["From"] = formataddr(("MARÉ", "shop@mare.swim"))
    msg["To"] = "julia@example.com"
    msg["Subject"] = "Resort '26 has landed"
    msg.set_content(
        "Resort '26 has landed. Six new pieces in colours pulled from the\n"
        "Pacific coastline at dusk. Shop at https://example.com\n"
    )
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


def s_mare_order(conn=None):
    """Order confirmation — line items with thumbnails, totals, ETA."""
    p = MARE_PRODUCT
    items = [
        (p[0], "M / Sandstone", "$98.00"),
        (p[4], "S / Ivory", "$78.00"),
    ]
    rows = ""
    for (img, name, price, color), variant, line_total in items:
        rows += f"""
          <tr>
            <td valign="top" style="padding:14px 0;border-bottom:1px solid #ece6dd" width="80">
              <img src="{img}" width="64" alt="{name}" style="display:block;width:64px;height:auto;border:0"/>
            </td>
            <td valign="top" style="padding:14px 14px;border-bottom:1px solid #ece6dd">
              <div style="font-size:13px;letter-spacing:1px;color:#1a1a1a;text-transform:uppercase">{name}</div>
              <div style="font-size:11px;letter-spacing:1px;color:#9c8d7a;margin-top:4px;text-transform:uppercase">{variant}</div>
            </td>
            <td valign="top" align="right" style="padding:14px 0;border-bottom:1px solid #ece6dd;font-size:13px;color:#1a1a1a">{line_total}</td>
          </tr>
        """
    body = f"""
      <tr><td class="mare-pad-md" style="padding:36px 48px 16px 48px;text-align:center">
        <div class="mare-eyebrow" style="font-size:11px;letter-spacing:4px;color:#9c8d7a;text-transform:uppercase">Order confirmed</div>
        <h1 class="mare-hero" style="font-family:Georgia,'Times New Roman',serif;font-weight:300;font-size:30px;line-height:1.15;margin:14px 0 6px 0;color:#1a1a1a">
          Thanks, Julia.
        </h1>
        <p style="font-size:14px;line-height:1.7;color:#52483b;margin:6px 0 0 0">
          Order <b>#MR-39204</b> &middot; Estimated arrival
          <b style="color:#1a1a1a">May 4 &mdash; May 6</b>
        </p>
      </td></tr>
      <tr><td class="mare-pad-md" style="padding:24px 48px 0 48px">
        <table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0">{rows}</table>
        <table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" style="margin-top:18px">
          <tr><td style="padding:6px 0;color:#9c8d7a;font-size:13px">Subtotal</td>
              <td align="right" style="padding:6px 0;color:#9c8d7a;font-size:13px">$176.00</td></tr>
          <tr><td style="padding:6px 0;color:#9c8d7a;font-size:13px">Shipping (express)</td>
              <td align="right" style="padding:6px 0;color:#9c8d7a;font-size:13px">$0.00</td></tr>
          <tr><td style="padding:14px 0 6px 0;border-top:1px solid #ece6dd;font-size:15px;font-weight:600;color:#1a1a1a">Total</td>
              <td align="right" style="padding:14px 0 6px 0;border-top:1px solid #ece6dd;font-size:15px;font-weight:600;color:#1a1a1a">$176.00</td></tr>
        </table>
      </td></tr>
      <tr><td align="center" class="mare-pad-md" style="padding:32px 48px 16px 48px">{_mare_button("Track your order")}</td></tr>
      <tr><td class="mare-pad-md" style="padding:0 48px 40px 48px;text-align:center">
        <p style="font-size:12px;line-height:1.7;color:#9c8d7a;margin:0">
          Shipping to Julia Park &middot; 218 Sunset Way, Apt 4 &middot; Venice CA 90291
        </p>
      </td></tr>
    """
    html = _mare_shell("Order #MR-39204 confirmed — arriving May 4–6", body)
    text = (
        "Order #MR-39204 confirmed.\n\n"
        "Lina Triangle Top — M / Sandstone — $98.00\n"
        "Reef Cheeky Bottom — S / Ivory — $78.00\n\n"
        "Total $176.00. Estimated arrival May 4–6.\n"
        "Track at https://example.com\n"
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("MARÉ", "orders@mare.swim"))
    msg["To"] = formataddr(("Julia Park", "julia@example.com"))
    msg["Subject"] = "Order #MR-39204 confirmed — thanks, Julia"
    msg.set_content(text)
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


def s_mare_cart(conn=None):
    """Cart abandonment — soft reminder, items in cart, related grid."""
    p = MARE_PRODUCT
    in_cart = [p[2], p[5]]
    rec = [p[1], p[3]]
    cart_rows = ""
    for img, name, price, color in in_cart:
        cart_rows += f"""
          <tr>
            <td valign="top" style="padding:14px 0;border-bottom:1px solid #ece6dd" width="80">
              <img src="{img}" width="64" alt="{name}" style="display:block;width:64px;height:auto;border:0"/>
            </td>
            <td valign="top" style="padding:14px 14px;border-bottom:1px solid #ece6dd">
              <div style="font-size:13px;letter-spacing:1px;color:#1a1a1a;text-transform:uppercase">{name}</div>
              <div style="font-size:11px;letter-spacing:1px;color:#9c8d7a;margin-top:4px;text-transform:uppercase">{color}</div>
            </td>
            <td valign="top" align="right" style="padding:14px 0;border-bottom:1px solid #ece6dd;font-size:13px;color:#1a1a1a">{price}</td>
          </tr>
        """
    body = f"""
      <tr><td class="mare-pad-md" style="padding:40px 48px 8px 48px;text-align:center">
        <div class="mare-eyebrow" style="font-size:11px;letter-spacing:4px;color:#9c8d7a;text-transform:uppercase">Still thinking?</div>
        <h1 class="mare-hero" style="font-family:Georgia,'Times New Roman',serif;font-weight:300;font-size:32px;line-height:1.15;margin:14px 0 4px 0;color:#1a1a1a">
          We saved your cart.
        </h1>
        <p style="font-size:14px;line-height:1.7;color:#52483b;margin:8px 16px 0 16px">
          Pieces sell out fast in resort drops. Yours are still here for now.
        </p>
      </td></tr>
      <tr><td class="mare-pad-md" style="padding:28px 48px 0 48px">
        <table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0">{cart_rows}</table>
      </td></tr>
      <tr><td align="center" class="mare-pad-md" style="padding:28px 48px 12px 48px">{_mare_button("Complete your order")}</td></tr>
      <tr><td class="mare-pad-md" style="padding:36px 48px 0 48px">
        <div style="font-size:11px;letter-spacing:3px;color:#9c8d7a;text-align:center;text-transform:uppercase;margin-bottom:18px">You may also like</div>
        <table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0">
          <tr>
            <td class="mare-stack" width="50%" valign="top" style="padding:0 8px 0 0">{_mare_product_card(*rec[0])}</td>
            <td class="mare-stack" width="50%" valign="top" style="padding:0 0 0 8px">{_mare_product_card(*rec[1])}</td>
          </tr>
        </table>
      </td></tr>
      <tr><td style="padding:36px 48px 40px 48px"></td></tr>
    """
    html = _mare_shell("Forget something? Your cart is waiting.", body)
    msg = EmailMessage()
    msg["From"] = formataddr(("MARÉ", "shop@mare.swim"))
    msg["To"] = "julia@example.com"
    msg["Subject"] = "Forget something? Your cart is waiting"
    msg.set_content("Your cart is still waiting at https://example.com")
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


def s_mare_sale(conn=None):
    """Flash sale — 48h banner, percentage, hero, 4-product grid."""
    p = MARE_PRODUCT
    body = f"""
      <tr><td bgcolor="#c97b5d" style="background:#c97b5d;padding:18px 24px;text-align:center">
        <div class="mare-banner-text" style="font-family:Helvetica,Arial,sans-serif;font-size:11px;letter-spacing:5px;color:#ffffff;text-transform:uppercase">
          48 Hours Only &middot; Ends Sunday Midnight
        </div>
      </td></tr>
      <tr><td>
        <img src="{MARE_HERO_BEACH}" width="600" alt="MARÉ resort sale"
             style="display:block;width:100%;height:auto;border:0"/>
      </td></tr>
      <tr><td class="mare-pad-md" style="padding:36px 48px 0 48px;text-align:center">
        <h1 class="mare-hero-xl" style="font-family:Georgia,'Times New Roman',serif;font-weight:300;font-size:48px;line-height:1;margin:0;color:#1a1a1a;letter-spacing:-1px">
          30% off
        </h1>
        <div style="font-size:13px;letter-spacing:5px;color:#9c8d7a;text-transform:uppercase;margin-top:14px">Sitewide</div>
        <p style="font-size:14px;line-height:1.7;color:#52483b;margin:18px 16px 28px 16px">
          The first sale of the season. Use code <b style="letter-spacing:1px">SHOREBREAK</b> at checkout.
        </p>
      </td></tr>
      <tr><td align="center" class="mare-pad-md" style="padding:0 48px 32px 48px">{_mare_button("Shop sale")}</td></tr>
      <tr><td class="mare-pad-md" style="padding:0 48px 40px 48px">
        <div style="font-size:11px;letter-spacing:3px;color:#9c8d7a;text-align:center;text-transform:uppercase;margin-bottom:18px">Bestsellers under $100</div>
        <table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0">
          <tr>
            <td class="mare-stack" width="50%" valign="top" style="padding:0 8px 16px 0">{_mare_product_card(*p[0])}</td>
            <td class="mare-stack" width="50%" valign="top" style="padding:0 0 16px 8px">{_mare_product_card(*p[3])}</td>
          </tr>
          <tr>
            <td class="mare-stack" width="50%" valign="top" style="padding:0 8px 0 0">{_mare_product_card(*p[1])}</td>
            <td class="mare-stack" width="50%" valign="top" style="padding:0 0 0 8px">{_mare_product_card(*p[4])}</td>
          </tr>
        </table>
      </td></tr>
    """
    html = _mare_shell(
        "30% off sitewide — 48 hours only with code SHOREBREAK",
        body,
        footer_note="Sale ends Sunday at midnight PT. Final sale items excluded.",
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("MARÉ", "shop@mare.swim"))
    msg["To"] = "julia@example.com"
    msg["Subject"] = "30% off sitewide — 48 hours only"
    msg.set_content(
        "30% off sitewide — 48 hours only.\n"
        "Use code SHOREBREAK at checkout. Sale ends Sunday at midnight PT.\n"
        "Shop at https://example.com\n"
    )
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


def s_mare_lookbook(conn=None):
    """Lookbook editorial — full-bleed photos, minimal copy, like a magazine."""
    body = f"""
      <tr><td>
        <img src="{MARE_HERO}" width="600" alt="MARÉ Resort '26 lookbook"
             style="display:block;width:100%;height:auto;border:0"/>
      </td></tr>
      <tr><td class="mare-pad-md" style="padding:48px 48px 0 48px;text-align:center">
        <div class="mare-eyebrow" style="font-size:11px;letter-spacing:5px;color:#9c8d7a;text-transform:uppercase">The Lookbook</div>
        <h1 class="mare-hero-xl" style="font-family:Georgia,'Times New Roman',serif;font-weight:300;font-size:42px;line-height:1.1;margin:18px 0 0 0;color:#1a1a1a;letter-spacing:-0.5px">
          Volume 04
        </h1>
        <p style="font-size:14px;line-height:1.8;color:#52483b;margin:22px 16px 0 16px;font-style:italic">
          &ldquo;Photographed in Tulum at first light, the Resort &rsquo;26
          collection borrows its colour story from the limestone cliffs and
          warm citrine sea.&rdquo;
        </p>
        <div style="font-size:11px;letter-spacing:3px;color:#9c8d7a;text-transform:uppercase;margin-top:18px">
          — Maya Olsson, Creative Director
        </div>
      </td></tr>
      <tr><td style="padding:36px 0 0 0">
        <img src="{MARE_HERO_2}" width="600" alt=""
             style="display:block;width:100%;height:auto;border:0"/>
      </td></tr>
      <tr><td class="mare-pad-md" style="padding:36px 48px 0 48px">
        <table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0">
          <tr>
            <td class="mare-stack" width="50%" valign="top" style="padding:0 4px 0 0">
              <img src="{MARE_PRODUCT[2][0]}" width="280" alt="" style="display:block;width:100%;height:auto;border:0"/>
            </td>
            <td class="mare-stack" width="50%" valign="top" style="padding:0 0 0 4px">
              <img src="{MARE_PRODUCT[5][0]}" width="280" alt="" style="display:block;width:100%;height:auto;border:0"/>
            </td>
          </tr>
        </table>
      </td></tr>
      <tr><td align="center" class="mare-pad-md" style="padding:40px 48px 48px 48px">{_mare_button("View the full lookbook")}</td></tr>
    """
    html = _mare_shell(
        "The Lookbook · Volume 04 — Resort '26",
        body,
        footer_note="The Lookbook ships monthly. Manage preferences any time.",
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("MARÉ Editorial", "editorial@mare.swim"))
    msg["To"] = "julia@example.com"
    msg["Subject"] = "The Lookbook · Volume 04"
    msg.set_content("The Lookbook · Volume 04 — view at https://example.com")
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


# ---------------------------------------------------------------------------
# Everyday work / personal scenarios — tools, services and notifications a
# typical inbox sees between the marketing ones. Visually unique per sender
# so the inbox preview reads as a realistic mix, not a wall of one template.
# ---------------------------------------------------------------------------


def s_linear_issue(conn=None):
    """Linear issue assigned — clean monochrome, structured fields."""
    html = (
        "<!doctype html><html><body style=\"margin:0;padding:0;"
        "background:#ffffff;font-family:Inter,-apple-system,sans-serif;"
        "color:#0f172a\">"
        "<table width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">"
        "<tr><td align=\"center\" style=\"padding:32px 16px\">"
        "<table width=\"560\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">"
        "<tr><td style=\"padding-bottom:24px\">"
        "<span style=\"display:inline-block;width:24px;height:24px;background:#5e6ad2;"
        "border-radius:6px;vertical-align:middle\"></span>"
        "<span style=\"font-weight:600;margin-left:10px;font-size:15px;color:#0f172a\">Linear</span>"
        "</td></tr>"
        "<tr><td style=\"font-size:13px;color:#64748b\">Marcus Chen assigned you</td></tr>"
        "<tr><td style=\"padding:8px 0 4px 0\">"
        "<span style=\"font-family:'SF Mono',ui-monospace,Menlo,monospace;font-size:12px;"
        "color:#5e6ad2;background:#eff1fb;padding:2px 6px;border-radius:4px\">ENG-3421</span>"
        "</td></tr>"
        "<tr><td style=\"font-size:18px;font-weight:600;padding:8px 0 16px 0\">"
        "Fix race condition in queue worker on retry</td></tr>"
        "<tr><td style=\"padding-bottom:24px;border-bottom:1px solid #e2e8f0\">"
        "<table width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">"
        "<tr>"
        "<td style=\"font-size:12px;color:#64748b;padding:6px 16px 6px 0;width:90px\">Status</td>"
        "<td style=\"font-size:13px;color:#0f172a\">"
        "<span style=\"display:inline-block;width:10px;height:10px;background:#f59e0b;"
        "border-radius:50%;vertical-align:middle;margin-right:6px\"></span>In Progress</td>"
        "</tr><tr>"
        "<td style=\"font-size:12px;color:#64748b;padding:6px 16px 6px 0\">Priority</td>"
        "<td style=\"font-size:13px;color:#0f172a\">High</td></tr>"
        "<tr>"
        "<td style=\"font-size:12px;color:#64748b;padding:6px 16px 6px 0\">Project</td>"
        "<td style=\"font-size:13px;color:#0f172a\">Reliability</td></tr>"
        "<tr>"
        "<td style=\"font-size:12px;color:#64748b;padding:6px 16px 6px 0\">Cycle</td>"
        "<td style=\"font-size:13px;color:#0f172a\">Cycle 14 · ends Apr 30</td></tr>"
        "</table></td></tr>"
        "<tr><td style=\"padding:24px 0\">"
        "<a href=\"https://example.com\" style=\"display:inline-block;background:#0f172a;"
        "color:#fff;padding:10px 18px;text-decoration:none;border-radius:6px;font-weight:500;"
        "font-size:13px\">Open in Linear</a></td></tr>"
        "</table></td></tr></table></body></html>"
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("Linear", "notifications@linear.app"))
    msg["To"] = "dev@example.com"
    msg["Subject"] = "ENG-3421 Fix race condition in queue worker on retry"
    msg.set_content(
        "Marcus Chen assigned you ENG-3421\n\n"
        "Fix race condition in queue worker on retry\n"
        "Status: In Progress · Priority: High · Cycle 14 ends Apr 30\n"
        "Open in Linear: https://example.com\n"
    )
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


def s_github_pr_review(conn=None):
    """GitHub PR review request — diff-style header, reviewer-list, files-changed."""
    html = (
        "<!doctype html><html><body style=\"margin:0;padding:0;"
        "background:#ffffff;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;"
        "color:#1f2328\">"
        "<table width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">"
        "<tr><td align=\"center\" style=\"padding:24px 16px\">"
        "<table width=\"600\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\" "
        "style=\"border:1px solid #d0d7de;border-radius:6px;overflow:hidden\">"
        "<tr><td style=\"padding:14px 16px;background:#f6f8fa;border-bottom:1px solid #d0d7de;"
        "font-size:12px;color:#656d76\">"
        "<b style=\"color:#1f2328\">harriet-l</b> requested your review on a pull request."
        "</td></tr>"
        "<tr><td style=\"padding:20px 16px 8px 16px\">"
        "<a href=\"https://example.com\" style=\"font-size:18px;font-weight:600;"
        "color:#0969da;text-decoration:none\">Use connection pooling for outbound webhooks</a>"
        "<div style=\"color:#656d76;font-size:13px;margin-top:4px\">"
        "<a href=\"https://example.com\" style=\"color:#656d76;text-decoration:none\">"
        "acme/webhook-relay</a> · <a href=\"https://example.com\" "
        "style=\"color:#656d76;text-decoration:none\">#1248</a></div></td></tr>"
        "<tr><td style=\"padding:8px 16px 16px 16px\">"
        "<span style=\"display:inline-block;background:#dafbe1;color:#1a7f37;"
        "padding:2px 7px;border-radius:9999px;font-size:11px;font-weight:500\">Open</span>"
        "<span style=\"display:inline-block;background:#ddf4ff;color:#0969da;"
        "padding:2px 7px;border-radius:9999px;font-size:11px;font-weight:500;"
        "margin-left:6px\">+421 −38</span>"
        "<span style=\"color:#656d76;font-size:12px;margin-left:10px\">12 files changed</span>"
        "</td></tr>"
        "<tr><td style=\"padding:0 16px 16px 16px;color:#1f2328;font-size:14px;"
        "line-height:1.55\">"
        "<p style=\"margin:0 0 12px 0\">Drops mean p99 webhook delivery time from "
        "880ms → 220ms in the staging soak. Reuses a 32-conn http2 pool keyed on "
        "destination authority. Reviewed by Hannah for the connection lifecycle.</p>"
        "<p style=\"margin:0;color:#656d76\">CI is green; needs your eyes on the "
        "graceful-shutdown code path.</p>"
        "</td></tr>"
        "<tr><td style=\"padding:0 16px 20px 16px\">"
        "<a href=\"https://example.com\" style=\"display:inline-block;background:#1f883d;"
        "color:#fff;padding:8px 16px;text-decoration:none;border-radius:6px;font-weight:500;"
        "font-size:14px;border:1px solid rgba(31,35,40,.15)\">View pull request</a></td></tr>"
        "</table>"
        "<p style=\"color:#656d76;font-size:12px;margin:14px 0 0\">"
        "You're receiving this because you were requested as a reviewer.</p>"
        "</td></tr></table></body></html>"
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("Harriet L (via GitHub)", "noreply@github.com"))
    msg["To"] = "dev@example.com"
    msg["Subject"] = "[acme/webhook-relay] Use connection pooling for outbound webhooks (#1248)"
    msg["Message-ID"] = make_msgid(domain="github.com")
    msg["List-ID"] = "acme/webhook-relay <webhook-relay.acme.github.com>"
    msg.set_content(
        "harriet-l requested your review on PR #1248\n\n"
        "Use connection pooling for outbound webhooks\n"
        "acme/webhook-relay #1248 · +421 −38 · 12 files changed\n"
        "View at https://example.com\n"
    )
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


def s_figma_comment(conn=None):
    """Figma comment — minimal, tan/cream design, frame thumbnail (solid swatch)."""
    html = (
        "<!doctype html><html><body style=\"margin:0;padding:0;"
        "background:#fafafa;font-family:Inter,-apple-system,sans-serif;color:#1a1a1a\">"
        "<table width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">"
        "<tr><td align=\"center\" style=\"padding:32px 16px\">"
        "<table width=\"540\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">"
        "<tr><td style=\"padding-bottom:20px\">"
        "<span style=\"display:inline-block;width:24px;height:24px;background:#000;"
        "border-radius:6px;vertical-align:middle;text-align:center;line-height:24px;"
        "color:#fff;font-weight:700;font-size:14px\">F</span>"
        "<span style=\"font-weight:600;margin-left:10px;font-size:15px\">Figma</span>"
        "</td></tr>"
        "<tr><td style=\"padding:0 0 16px 0;font-size:13px;color:#737373\">"
        "<b style=\"color:#1a1a1a\">Sarah Mendez</b> commented on "
        "<b style=\"color:#1a1a1a\">Resort '26 / Hero / Frame 12</b>"
        "</td></tr>"
        "<tr><td style=\"padding:0 0 16px 0\">"
        "<table width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\" "
        "style=\"border:1px solid #e5e5e5;border-radius:8px;overflow:hidden\">"
        "<tr><td style=\"background:#f5e9d8;height:200px\">&nbsp;</td></tr>"
        "<tr><td style=\"padding:12px 14px;font-size:12px;color:#737373;background:#fff\">"
        "Resort '26 / Hero / Frame 12</td></tr></table></td></tr>"
        "<tr><td style=\"padding:0 0 16px 0\">"
        "<table cellpadding=\"0\" cellspacing=\"0\" border=\"0\">"
        "<tr><td style=\"width:32px;vertical-align:top\">"
        "<span style=\"display:inline-block;width:28px;height:28px;background:#e879f9;"
        "border-radius:50%;text-align:center;line-height:28px;color:#fff;font-weight:600;"
        "font-size:13px\">SM</span></td>"
        "<td style=\"padding-left:8px\">"
        "<div style=\"font-size:13px;color:#1a1a1a;font-weight:600\">Sarah Mendez "
        "<span style=\"font-weight:400;color:#a3a3a3;font-size:12px\">· just now</span></div>"
        "<div style=\"font-size:14px;color:#1a1a1a;margin-top:6px;line-height:1.5\">"
        "@dev should the model placement match the SS25 hero spec? It feels a bit "
        "low compared to what we landed on last season. Happy to do a quick Loom if easier.</div>"
        "</td></tr></table></td></tr>"
        "<tr><td style=\"padding:8px 0 0 0\">"
        "<a href=\"https://example.com\" style=\"display:inline-block;background:#0d99ff;"
        "color:#fff;padding:9px 16px;text-decoration:none;border-radius:6px;font-weight:500;"
        "font-size:13px\">Reply in Figma</a></td></tr>"
        "</table></td></tr></table></body></html>"
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("Figma", "no-reply@figma.com"))
    msg["To"] = "dev@example.com"
    msg["Subject"] = "Sarah Mendez commented on Resort '26 / Hero / Frame 12"
    msg.set_content(
        "Sarah Mendez commented on Resort '26 / Hero / Frame 12\n\n"
        "@dev should the model placement match the SS25 hero spec?\n"
        "Reply: https://example.com\n"
    )
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


def s_google_doc_comment(conn=None):
    """Google Docs comment — clean Google styling, document title, comment."""
    html = (
        "<!doctype html><html><body style=\"margin:0;padding:0;"
        "background:#ffffff;font-family:'Google Sans',Roboto,Arial,sans-serif;"
        "color:#202124\">"
        "<table width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">"
        "<tr><td align=\"center\" style=\"padding:32px 16px\">"
        "<table width=\"560\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">"
        "<tr><td style=\"padding-bottom:24px\">"
        "<span style=\"font-size:14px;color:#5f6368\">A new comment on a Google Doc</span>"
        "</td></tr>"
        "<tr><td style=\"padding:0 0 12px 0\">"
        "<table cellpadding=\"0\" cellspacing=\"0\" border=\"0\">"
        "<tr><td style=\"vertical-align:middle;width:32px\">"
        "<span style=\"display:inline-block;width:28px;height:28px;background:#1a73e8;"
        "border-radius:50%;text-align:center;line-height:28px;color:#fff;font-weight:500\">M</span>"
        "</td><td style=\"padding-left:10px;vertical-align:middle\">"
        "<span style=\"font-weight:500;font-size:14px\">Maya Olsson</span></td></tr>"
        "</table></td></tr>"
        "<tr><td style=\"font-size:14px;color:#202124;line-height:1.6;padding:0 0 14px 0\">"
        "Pulled the campaign captions in. Last paragraph still needs a fact-check on "
        "the Carvico recycled-nylon claim — flagged @dev to confirm."
        "</td></tr>"
        "<tr><td style=\"padding:14px 16px;background:#fef7e0;border-radius:8px;font-size:13px;"
        "color:#3c4043;line-height:1.6\">"
        "<i>&ldquo;made in small batches from recycled-nylon Carvico Vita and trimmed with "
        "handcrafted shell beads&rdquo;</i>"
        "</td></tr>"
        "<tr><td style=\"padding:20px 0 8px 0;font-size:14px;color:#5f6368\">"
        "Document: <a href=\"https://example.com\" style=\"color:#1a73e8;text-decoration:none\">"
        "MARÉ Resort '26 — Email Copy.docx</a></td></tr>"
        "<tr><td style=\"padding:14px 0 0 0\">"
        "<a href=\"https://example.com\" style=\"display:inline-block;background:#1a73e8;"
        "color:#fff;padding:9px 18px;text-decoration:none;border-radius:4px;font-weight:500;"
        "font-size:14px\">Open</a></td></tr>"
        "</table></td></tr></table></body></html>"
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("Maya Olsson (via Google Docs)", "comments-noreply@docs.google.com"))
    msg["To"] = "dev@example.com"
    msg["Subject"] = "Maya commented on \"MARÉ Resort '26 — Email Copy.docx\""
    msg.set_content(
        "Maya Olsson commented on MARÉ Resort '26 — Email Copy.docx\n\n"
        "Pulled the campaign captions in. Last paragraph still needs a fact-check\n"
        "on the Carvico recycled-nylon claim — flagged @dev to confirm.\n\n"
        "Open: https://example.com\n"
    )
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


def s_substack(conn=None):
    """Substack-style longform newsletter — header, headline, kicker, body, share row."""
    html = (
        "<!doctype html><html><body style=\"margin:0;padding:0;"
        "background:#fafaf7;font-family:Georgia,'Times New Roman',serif;color:#1a1a1a\">"
        "<table width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">"
        "<tr><td align=\"center\" style=\"padding:32px 16px\">"
        "<table width=\"600\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\" "
        "style=\"background:#ffffff;border:1px solid #ebe6dc\">"
        "<tr><td style=\"padding:32px 40px 8px 40px;text-align:center;"
        "border-bottom:1px solid #ebe6dc\">"
        "<div style=\"font-family:Helvetica,Arial,sans-serif;font-size:11px;letter-spacing:3px;"
        "color:#a8a195;text-transform:uppercase;margin-bottom:6px\">Issue 142</div>"
        "<div style=\"font-size:24px;font-weight:400;color:#1a1a1a;letter-spacing:-0.3px\">"
        "Field Notes</div>"
        "<div style=\"font-family:Helvetica,Arial,sans-serif;font-size:11px;letter-spacing:1.5px;"
        "color:#a8a195;text-transform:uppercase;margin-top:10px;padding-bottom:24px\">"
        "by Anya Petrov &middot; Sunday, April 27</div>"
        "</td></tr>"
        "<tr><td style=\"padding:36px 40px 4px 40px\">"
        "<h1 style=\"font-size:32px;font-weight:400;line-height:1.2;margin:0;color:#1a1a1a;"
        "letter-spacing:-0.5px\">The case for boring infrastructure</h1>"
        "<p style=\"font-family:Helvetica,Arial,sans-serif;font-size:13px;color:#a8a195;"
        "margin:14px 0 0 0;font-style:italic\">A 7-minute read on why the most reliable "
        "stacks I've worked on were also the most boring.</p>"
        "</td></tr>"
        "<tr><td style=\"padding:24px 40px 0 40px;font-size:17px;line-height:1.7;color:#262626\">"
        "<p style=\"margin:0 0 18px 0\">Every place I've worked has had a someone who "
        "wanted to rewrite the queue. Maybe it's Kafka, maybe it's a homebrew Postgres-LISTEN "
        "thing, maybe it's a SaaS bus that one team uses for one workflow.</p>"
        "<p style=\"margin:0 0 18px 0\">The boring teams kept the queue. The teams that "
        "rewrote the queue spent a quarter rewriting the queue.</p>"
        "<p style=\"margin:0;color:#a8a195\"><i>Continue reading on the web &rarr;</i></p>"
        "</td></tr>"
        "<tr><td style=\"padding:32px 40px 32px 40px\">"
        "<a href=\"https://example.com\" style=\"display:inline-block;background:#1a1a1a;"
        "color:#fff;padding:12px 28px;text-decoration:none;font-family:Helvetica,Arial,sans-serif;"
        "font-size:13px;letter-spacing:0.5px;font-weight:500\">Read on Substack</a></td></tr>"
        "<tr><td style=\"padding:0 40px 32px 40px;border-top:1px solid #ebe6dc\">"
        "<p style=\"font-family:Helvetica,Arial,sans-serif;font-size:11px;color:#a8a195;"
        "letter-spacing:0.5px;margin:20px 0 0 0;text-align:center\">"
        "&copy; 2026 Anya Petrov &middot; "
        "<a href=\"https://example.com\" style=\"color:#a8a195\">Unsubscribe</a></p>"
        "</td></tr>"
        "</table></td></tr></table></body></html>"
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("Anya Petrov · Field Notes", "fieldnotes@substack.com"))
    msg["To"] = "dev@example.com"
    msg["Subject"] = "The case for boring infrastructure"
    msg.set_content(
        "Field Notes — Issue 142 — by Anya Petrov\n\n"
        "The case for boring infrastructure (7-minute read)\n\n"
        "Read on Substack: https://example.com\n"
    )
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


def s_stripe_payment(conn=None):
    """Stripe payment received — green check, amount, customer, line items."""
    html = (
        "<!doctype html><html><body style=\"margin:0;padding:0;"
        "background:#f5f5f5;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;"
        "color:#1a1f36\">"
        "<table width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">"
        "<tr><td align=\"center\" style=\"padding:32px 16px\">"
        "<table width=\"560\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\" "
        "style=\"background:#fff;border-radius:8px;box-shadow:0 1px 3px rgba(50,50,93,.1)\">"
        "<tr><td style=\"padding:36px 36px 16px 36px\">"
        "<div style=\"font-size:14px;color:#635bff;font-weight:600;letter-spacing:0.3px\">"
        "Stripe</div>"
        "</td></tr>"
        "<tr><td style=\"padding:0 36px 4px 36px\">"
        "<div style=\"font-size:14px;color:#697386\">Payment received</div>"
        "<div style=\"font-size:32px;font-weight:700;color:#1a1f36;margin-top:6px\">"
        "$2,400.00 <span style=\"font-size:14px;font-weight:400;color:#697386\">USD</span></div>"
        "</td></tr>"
        "<tr><td style=\"padding:24px 36px 8px 36px\">"
        "<table width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">"
        "<tr><td style=\"padding:8px 0;border-bottom:1px solid #e3e8ee\">"
        "<div style=\"font-size:12px;color:#697386\">Customer</div>"
        "<div style=\"font-size:14px;color:#1a1f36;margin-top:2px\">Acme Co · billing@acme.example</div></td></tr>"
        "<tr><td style=\"padding:8px 0;border-bottom:1px solid #e3e8ee\">"
        "<div style=\"font-size:12px;color:#697386\">Invoice</div>"
        "<div style=\"font-size:14px;color:#1a1f36;margin-top:2px\">INV-04821 · April retainer</div></td></tr>"
        "<tr><td style=\"padding:8px 0;border-bottom:1px solid #e3e8ee\">"
        "<div style=\"font-size:12px;color:#697386\">Method</div>"
        "<div style=\"font-size:14px;color:#1a1f36;margin-top:2px\">Visa ending 4242</div></td></tr>"
        "<tr><td style=\"padding:8px 0\">"
        "<div style=\"font-size:12px;color:#697386\">Net (after fees)</div>"
        "<div style=\"font-size:14px;color:#1a1f36;margin-top:2px\">$2,330.40 (fees $69.60)</div></td></tr>"
        "</table></td></tr>"
        "<tr><td style=\"padding:24px 36px 36px 36px\">"
        "<a href=\"https://example.com\" style=\"display:inline-block;background:#635bff;"
        "color:#fff;padding:10px 18px;text-decoration:none;border-radius:6px;font-weight:600;"
        "font-size:14px\">View payment</a></td></tr>"
        "</table></td></tr></table></body></html>"
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("Stripe", "no-reply@stripe.com"))
    msg["To"] = "dev@example.com"
    msg["Subject"] = "You received a payment of $2,400.00 from Acme Co"
    msg.set_content(
        "Payment received: $2,400.00 USD from Acme Co\n"
        "Invoice INV-04821 · April retainer\n"
        "Net (after fees): $2,330.40\n"
    )
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


def s_meeting_reminder(conn=None):
    """Calendar 'starts in 15 min' reminder — event card with attendees."""
    when = (datetime.now(timezone.utc) + timedelta(minutes=15)).astimezone()
    html = (
        "<!doctype html><html><body style=\"margin:0;padding:0;"
        "background:#ffffff;font-family:'Google Sans',Roboto,Arial,sans-serif;color:#202124\">"
        "<table width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">"
        "<tr><td align=\"center\" style=\"padding:32px 16px\">"
        "<table width=\"540\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">"
        "<tr><td style=\"padding-bottom:18px;font-size:14px;color:#5f6368\">"
        "Reminder · meeting starts in 15 minutes"
        "</td></tr>"
        "<tr><td style=\"padding:0 0 14px 0\">"
        "<table width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\" "
        "style=\"border:1px solid #dadce0;border-radius:8px\"><tr>"
        "<td style=\"width:6px;background:#1a73e8;border-radius:8px 0 0 8px\">&nbsp;</td>"
        "<td style=\"padding:18px 18px\">"
        "<div style=\"font-size:18px;font-weight:500\">1:1 with Marcus</div>"
        f"<div style=\"font-size:13px;color:#5f6368;margin-top:6px\">"
        f"{when.strftime('%A, %b %-d')} &middot; "
        f"{when.strftime('%-I:%M %p')} – {(when + timedelta(minutes=30)).strftime('%-I:%M %p')}</div>"
        "<div style=\"font-size:13px;color:#5f6368;margin-top:4px\">Google Meet</div>"
        "<div style=\"margin-top:14px\">"
        "<a href=\"https://example.com\" style=\"display:inline-block;background:#1a73e8;"
        "color:#fff;padding:8px 16px;text-decoration:none;border-radius:4px;font-weight:500;"
        "font-size:13px\">Join with Google Meet</a></div>"
        "</td></tr></table></td></tr>"
        "<tr><td style=\"padding:14px 0 0 0;font-size:13px;color:#5f6368\">"
        "Going: Marcus Chen, you · Maybe: Hannah Brooks"
        "</td></tr>"
        "</table></td></tr></table></body></html>"
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("Google Calendar", "calendar-noreply@google.com"))
    msg["To"] = "dev@example.com"
    msg["Subject"] = f"Reminder: 1:1 with Marcus @ {when.strftime('%-I:%M %p')} (15 min)"
    msg.set_content(
        f"Reminder — meeting starts in 15 minutes.\n\n"
        f"1:1 with Marcus\n"
        f"{when.strftime('%A, %b %-d at %-I:%M %p')}\n"
        f"Join: https://example.com\n"
    )
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


def s_apple_receipt(conn=None):
    """Apple App Store receipt — clean Apple typography, line items, Apple ID footer."""
    html = (
        "<!doctype html><html><body style=\"margin:0;padding:0;"
        "background:#ffffff;font-family:-apple-system,BlinkMacSystemFont,'SF Pro Display',sans-serif;"
        "color:#1d1d1f\">"
        "<table width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">"
        "<tr><td align=\"center\" style=\"padding:48px 16px\">"
        "<table width=\"500\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">"
        "<tr><td align=\"center\" style=\"padding-bottom:32px\">"
        "<span style=\"font-size:36px;color:#000\"></span>"
        "</td></tr>"
        "<tr><td style=\"padding-bottom:8px;font-size:11px;letter-spacing:1.5px;"
        "color:#86868b;text-transform:uppercase\">Receipt</td></tr>"
        "<tr><td style=\"padding-bottom:32px;font-size:24px;font-weight:600;color:#1d1d1f\">"
        "Your receipt from Apple.</td></tr>"
        "<tr><td style=\"padding-bottom:14px;border-bottom:1px solid #d2d2d7\">"
        "<table width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">"
        "<tr>"
        "<td style=\"font-size:12px;color:#86868b;padding:6px 0;width:50%\">Apple ID</td>"
        "<td align=\"right\" style=\"font-size:12px;color:#1d1d1f;padding:6px 0\">"
        "matt@example.com</td></tr>"
        "<tr>"
        "<td style=\"font-size:12px;color:#86868b;padding:6px 0\">Date</td>"
        "<td align=\"right\" style=\"font-size:12px;color:#1d1d1f;padding:6px 0\">"
        "Apr 27, 2026</td></tr>"
        "<tr>"
        "<td style=\"font-size:12px;color:#86868b;padding:6px 0\">Order ID</td>"
        "<td align=\"right\" style=\"font-size:12px;color:#1d1d1f;padding:6px 0\">"
        "ML82PQ9XN3</td></tr>"
        "</table></td></tr>"
        "<tr><td style=\"padding:18px 0;border-bottom:1px solid #d2d2d7\">"
        "<table width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\"><tr>"
        "<td style=\"padding-right:12px;width:54px\">"
        "<span style=\"display:inline-block;width:48px;height:48px;background:#fbbf24;"
        "border-radius:12px;text-align:center;line-height:48px;color:#fff;font-weight:700;"
        "font-size:22px\">T</span></td>"
        "<td style=\"padding-right:8px\">"
        "<div style=\"font-size:14px;font-weight:500;color:#1d1d1f\">Things 3</div>"
        "<div style=\"font-size:12px;color:#86868b;margin-top:2px\">"
        "Cultured Code GmbH &middot; macOS &middot; In-app purchase</div>"
        "<div style=\"font-size:11px;color:#0070c9;margin-top:2px\">Write a review</div>"
        "</td>"
        "<td align=\"right\" valign=\"top\" style=\"font-size:14px;color:#1d1d1f\">$49.99</td>"
        "</tr></table></td></tr>"
        "<tr><td style=\"padding:14px 0\">"
        "<table width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">"
        "<tr>"
        "<td style=\"font-size:13px;color:#1d1d1f;padding:4px 0\">Subtotal</td>"
        "<td align=\"right\" style=\"font-size:13px;color:#1d1d1f;padding:4px 0\">$49.99</td></tr>"
        "<tr>"
        "<td style=\"font-size:13px;color:#1d1d1f;padding:4px 0\">Tax</td>"
        "<td align=\"right\" style=\"font-size:13px;color:#1d1d1f;padding:4px 0\">$4.50</td></tr>"
        "<tr>"
        "<td style=\"font-size:15px;font-weight:600;color:#1d1d1f;padding:10px 0 4px 0;"
        "border-top:1px solid #d2d2d7\">Total</td>"
        "<td align=\"right\" style=\"font-size:15px;font-weight:600;color:#1d1d1f;"
        "padding:10px 0 4px 0;border-top:1px solid #d2d2d7\">$54.49</td></tr>"
        "</table></td></tr>"
        "<tr><td style=\"padding:24px 0 0 0;font-size:11px;color:#86868b;line-height:1.6\">"
        "Privacy: We use a Subject Identifier &mdash; a unique alphanumeric value &mdash; to "
        "open and improve the App Store and other Apple services. Get help, manage subscriptions, "
        "and more at <a href=\"https://example.com\" style=\"color:#0070c9\">reportaproblem.apple.com</a>."
        "</td></tr>"
        "</table></td></tr></table></body></html>"
    )
    msg = EmailMessage()
    msg["From"] = formataddr(("Apple", "no_reply@email.apple.com"))
    msg["To"] = "matt@example.com"
    msg["Subject"] = "Your receipt from Apple"
    msg.set_content(
        "Your receipt from Apple.\n\n"
        "Things 3 — Cultured Code GmbH (macOS, In-app purchase)\n"
        "$49.99 + $4.50 tax = $54.49\n"
        "Order ML82PQ9XN3\n"
    )
    msg.add_alternative(html, subtype="html")
    send(msg, conn)


def s_burst(conn=None, count: int = 50):
    """Fire `count` messages quickly using a single connection."""
    senders = [
        "alice@example.com",
        "bob@example.com",
        "carol@example.com",
        "dan@example.com",
        "eve@example.com",
    ]
    subjects = [
        "Quick question",
        "Re: yesterday",
        "Standup notes",
        "📈 Numbers are up",
        "[ALERT] queue depth high",
        "Budget review",
        "Meeting moved",
        "PR ready for review",
    ]
    own_conn = conn is None
    s = conn or open_session()
    try:
        for i in range(count):
            msg = EmailMessage()
            msg["From"] = random.choice(senders)
            msg["To"] = "dev@example.com"
            msg["Subject"] = f"{random.choice(subjects)} #{i + 1:03d}"
            if "Date" not in msg:
                msg["Date"] = formatdate(localtime=True)
            if "Message-ID" not in msg:
                msg["Message-ID"] = make_msgid(domain="mailboxultra.local")
            msg.set_content(f"Burst message {i + 1} of {count}.")
            s.send_message(msg)
        print(f"  burst sent: {count} messages", file=sys.stderr)
    finally:
        if own_conn:
            s.quit()


# ---------------------------------------------------------------------------
# Registry + CLI
# ---------------------------------------------------------------------------


SCENARIOS: Dict[str, Callable] = {
    # Order is also the firing order in default mode. Curated to look like a
    # plausible day's inbox: marketing/ecom interleaved with work tooling
    # (Linear/GitHub/Figma/Slack-adjacent), transactional (Stripe/Apple),
    # newsletters, and personal/calendar reminders.
    "plain": s_plain,
    "welcome": s_welcome,
    "linear": s_linear_issue,
    "mare-welcome": s_mare_welcome,
    "github-pr-review": s_github_pr_review,
    "receipt": s_receipt,
    "figma": s_figma_comment,
    "mare-drop": s_mare_drop,
    "shipping": s_shipping,
    "google-doc": s_google_doc_comment,
    "stripe-payment": s_stripe_payment,
    "mare-order": s_mare_order,
    "password-reset": s_password_reset,
    "meeting-reminder": s_meeting_reminder,
    "substack": s_substack,
    "mare-cart": s_mare_cart,
    "newsletter": s_newsletter,
    "sale": s_sale_alert,
    "apple-receipt": s_apple_receipt,
    "mare-sale": s_mare_sale,
    "github": s_github_notification,
    "ci-failure": s_ci_failure,
    "monitor": s_monitor_alert,
    "survey": s_survey,
    "calendar": s_calendar_invite,
    "mare-lookbook": s_mare_lookbook,
    "with-pdf": s_with_pdf,
    "with-image": s_with_image,
    "text-attach": s_text_with_attachment,
    "many-recipients": s_many_recipients,
    "unicode": s_unicode,
    "encoded-subject": s_encoded_subject,
    "long-subject": s_long_subject,
    "long-body": s_long_body,
    "no-subject": s_no_subject,
    "html-only": s_html_only,
    "reply-thread": s_reply_thread,
    "dark-mode": s_dark_mode_aware,
    "marketing": s_marketing_image_heavy,
    "burst": s_burst,
}


def main(argv: Optional[List[str]] = None) -> int:
    p = argparse.ArgumentParser(
        description="Fire varied SMTP messages at MailBox Ultra for previewing + dev.",
        epilog="If no scenarios are given, every scenario except 'burst' runs once.",
    )
    p.add_argument("scenarios", nargs="*", help="scenario names to run")
    p.add_argument("--list", action="store_true", help="list available scenarios")
    p.add_argument(
        "--delay",
        type=float,
        default=DEFAULT_DELAY,
        help="seconds between sends in batch mode (default %(default)s)",
    )
    p.add_argument(
        "-n",
        "--count",
        type=int,
        default=50,
        help="message count for the burst scenario (default %(default)s)",
    )
    p.add_argument(
        "--all",
        action="store_true",
        help="include 'burst' when running every scenario",
    )
    args = p.parse_args(argv)

    if args.list:
        width = max(len(n) for n in SCENARIOS)
        for name, fn in SCENARIOS.items():
            doc = (fn.__doc__ or "").strip().splitlines()
            summary = doc[0] if doc else ""
            print(f"  {name:<{width}}  {summary}")
        return 0

    if args.scenarios:
        names = args.scenarios
        unknown = [n for n in names if n not in SCENARIOS]
        if unknown:
            print(f"unknown scenario(s): {', '.join(unknown)}", file=sys.stderr)
            print(f"run --list to see options", file=sys.stderr)
            return 2
    else:
        names = [n for n in SCENARIOS if args.all or n != "burst"]

    print(f"-> {HOST}:{PORT}", file=sys.stderr)
    if AUTH:
        print(f"   AUTH as {AUTH.split(':', 1)[0]}", file=sys.stderr)

    # Reuse a single connection for the batch — much faster.
    try:
        with open_session() as conn:
            for name in names:
                fn = SCENARIOS[name]
                if name == "burst":
                    s_burst(conn, count=args.count)
                else:
                    fn(conn)
                if name != names[-1]:
                    time.sleep(args.delay)
    except (ConnectionRefusedError, OSError) as e:
        print(
            f"error: could not connect to {HOST}:{PORT} ({e}). "
            f"Is the app running?",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
