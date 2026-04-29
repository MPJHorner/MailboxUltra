---
title: "Web UI"
description: "Tabs, formatters, keyboard shortcuts, and theme controls."
slug: web-ui
---

# Web UI

Open `http://127.0.0.1:8025` once `mailbox-ultra` is running. The UI is rendered entirely client-side from the JSON API and SSE stream — there is no template engine on the server.

## Layout

- **Topbar** — brand, SMTP URL pill, search box, message counter, relay configuration pill, pause toggle, theme toggle, shortcuts help, clear-all, connection status dot.
- **Left pane** — live list of captured messages, newest first. Filter via the search box.
- **Right pane** — detail view for the selected message, with six tabs.

## Tabs

| Tab | Shows |
|---|---|
| **HTML** | Sandboxed iframe rendering of the HTML body. No script execution, no parent-document access. Device-size preview switcher (Desktop / iPad 820 / Mobile 390) sits above the frame, and the frame fills the available height. |
| **Text** | Plain-text alternative if present, raw text of the body otherwise. |
| **Headers** | Every header in the order it arrived, including duplicates. |
| **Attachments** | One row per part with content type, size, and a one-click download. |
| **Source** | Raw RFC 822 source as bytes hit the server. Useful for debugging custom MIME. |
| **Release** | Resend this captured message to a target SMTP server. Pre-fills the global relay URL when one is configured. |

## Keyboard shortcuts

| Key | Action |
|---|---|
| <kbd>j</kbd> / <kbd>↓</kbd> | Next message |
| <kbd>k</kbd> / <kbd>↑</kbd> | Previous message |
| <kbd>g</kbd> / <kbd>G</kbd> | Jump to newest / oldest |
| <kbd>/</kbd> | Focus search |
| <kbd>1</kbd>–<kbd>6</kbd> | Switch tabs |
| <kbd>p</kbd> | Pause / resume the live display |
| <kbd>d</kbd> | Delete the current message |
| <kbd>Shift</kbd>+<kbd>X</kbd> | Clear all (confirmation prompt) |
| <kbd>t</kbd> | Toggle theme |
| <kbd>?</kbd> | Show this help |
| <kbd>Esc</kbd> | Close dialog / blur search |

Modifier keys (<kbd>⌘</kbd>, <kbd>Ctrl</kbd>, <kbd>Alt</kbd>) are never intercepted, so <kbd>⌘</kbd>+<kbd>C</kbd> still copies and <kbd>⌘</kbd>+<kbd>R</kbd> still reloads.

## Theme

Click the moon icon or press <kbd>t</kbd>. The choice is persisted in `localStorage` under the `mbu-theme` key, shared between the embedded UI and this docs site.

## Pause

While paused, the SSE stream keeps draining in the background but new messages do not appear in the list. Unpause and the list snaps to the latest state. Useful when you're reading a captured email and don't want it to scroll away.

## Search

The search box matches against subject, parsed and envelope `from`/`to` addresses. It runs against the in-memory list — no server round-trip — so it stays instant even with thousands of messages.

## Device preview

The HTML tab includes a row of three buttons above the rendered iframe:

| Button | Width | Use |
|---|---|---|
| **Desktop** | full | Edge-to-edge preview, the way most clients render at full window. |
| **iPad** | 820 px | Centered frame at tablet width, with a soft drop-shadow around the device. |
| **Mobile** | 390 px | iPhone-class width. Hits the `@media` breakpoints in well-built responsive emails. |

The iframe fills the available height of the detail pane regardless of which size you pick, so long emails scroll inside the frame rather than the surrounding chrome. Your selection is remembered as you click between captured messages.

## Sandboxing

The HTML preview iframe ships with `sandbox="allow-popups"` and a Blob URL source. That means:

- No JavaScript executes inside the captured email.
- The captured email cannot reach the parent document or read your localStorage.
- Links can still be opened in a new tab.
- Images and styles render normally.

This is the same posture Gmail and Mailpit take when previewing untrusted HTML.
