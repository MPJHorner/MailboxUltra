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
    "plain": s_plain,
    "welcome": s_welcome,
    "receipt": s_receipt,
    "shipping": s_shipping,
    "password-reset": s_password_reset,
    "newsletter": s_newsletter,
    "sale": s_sale_alert,
    "github": s_github_notification,
    "ci-failure": s_ci_failure,
    "monitor": s_monitor_alert,
    "survey": s_survey,
    "calendar": s_calendar_invite,
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
