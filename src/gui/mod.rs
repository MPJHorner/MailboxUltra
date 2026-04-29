//! egui front-end. Owns view state, draws the toolbar / inbox / detail
//! panes, and forwards user actions to the [`ServerHandle`].
//!
//! Phase 4 wires the shell + inbox + a placeholder detail pane. The real
//! tabs (Phase 5), native HTML preview (Phase 6), and Preferences /
//! Relay / Help windows (Phase 7) build on this skeleton.

pub mod detail;
pub mod fonts;
pub mod help_window;
pub mod inbox;
#[cfg(target_os = "macos")]
pub mod native_html;
pub mod relay_window;
pub mod repaint;
pub mod settings_window;
pub mod theme;
pub mod toasts;
pub mod toolbar;

use std::sync::Arc;

use egui::Key;
use uuid::Uuid;

use crate::message::Message;
use crate::server::ServerHandle;
use crate::settings::PersistentSettings;

use self::detail::{DetailContext, DetailState, DetailTab};
use self::help_window::HelpWindowState;
use self::inbox::{InboxAction, InboxRenderContext, InboxState};
use self::relay_window::RelayWindowState;
use self::repaint::StoreSubscription;
use self::settings_window::SettingsWindowState;
use self::toasts::ToastList;
use self::toolbar::ToolbarContext;

pub struct MailboxApp {
    server: Arc<ServerHandle>,
    settings: PersistentSettings,
    subscription: StoreSubscription,
    toasts: ToastList,

    inbox: InboxState,
    detail: DetailState,

    paused: bool,
    list_snapshot: Vec<Message>,
    pending_focus_search: bool,
    last_applied_theme: crate::settings::Theme,

    settings_window: SettingsWindowState,
    relay_window: RelayWindowState,
    help_window: HelpWindowState,
    confirm_clear: bool,

    #[cfg(target_os = "macos")]
    native_html: Option<native_html::NativeHtmlView>,
}

impl MailboxApp {
    pub fn new(server: Arc<ServerHandle>, cc: &eframe::CreationContext<'_>) -> Self {
        let settings = server.settings();
        fonts::install(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx, settings.theme);
        let subscription = StoreSubscription::new(
            server.clone(),
            tokio::runtime::Handle::current(),
            cc.egui_ctx.clone(),
        );
        #[cfg(target_os = "macos")]
        let native_html = match native_html::NativeHtmlView::attach(cc) {
            Some(v) => Some(v),
            None => {
                tracing::warn!("could not attach native HTML view; HTML tab will fall back");
                None
            }
        };
        Self {
            server,
            settings: settings.clone(),
            subscription,
            toasts: ToastList::default(),
            inbox: InboxState::default(),
            detail: DetailState::default(),
            paused: false,
            list_snapshot: Vec::new(),
            pending_focus_search: false,
            last_applied_theme: settings.theme,
            settings_window: SettingsWindowState::default(),
            relay_window: RelayWindowState::default(),
            help_window: HelpWindowState::default(),
            confirm_clear: false,
            #[cfg(target_os = "macos")]
            native_html,
        }
    }

    fn smtp_url(&self) -> String {
        format!("smtp://{}", self.server.smtp_addr())
    }

    fn refresh_snapshot(&mut self) {
        if self.paused {
            return;
        }
        let store = self.server.store();
        let limit = store.capacity();
        self.list_snapshot = store.list(limit);

        // Auto-select the newest message when nothing's selected. Mirrors the
        // "click the inbox, see the latest message" behaviour Mac mail apps
        // have, and means new captured mail is immediately readable without
        // a click.
        if self.inbox.selected_id.is_none() {
            if let Some(first) = self.list_snapshot.first() {
                self.inbox.selected_id = Some(first.id);
            }
        }
    }

    fn handle_inbox_keys(&mut self, ctx: &egui::Context) {
        // Suppress when a text input has focus.
        if ctx.memory(|m| m.focused().is_some()) {
            return;
        }
        ctx.input(|i| {
            let visible: Vec<Uuid> = self
                .list_snapshot
                .iter()
                .filter(|m| self.inbox.matches(m))
                .map(|m| m.id)
                .collect();
            if visible.is_empty() {
                return;
            }
            let current = self
                .inbox
                .selected_id
                .and_then(|id| visible.iter().position(|x| *x == id));

            if i.key_pressed(Key::J) || i.key_pressed(Key::ArrowDown) {
                let next = current.map(|p| (p + 1).min(visible.len() - 1)).unwrap_or(0);
                let id = visible.get(next).copied();
                self.inbox.selected_id = id;
                self.inbox.scroll_to = id;
            }
            if i.key_pressed(Key::K) || i.key_pressed(Key::ArrowUp) {
                let prev = current.map(|p| p.saturating_sub(1)).unwrap_or(0);
                let id = visible.get(prev).copied();
                self.inbox.selected_id = id;
                self.inbox.scroll_to = id;
            }
            if i.key_pressed(Key::G) {
                let id = if i.modifiers.shift {
                    visible.last().copied()
                } else {
                    visible.first().copied()
                };
                self.inbox.selected_id = id;
                self.inbox.scroll_to = id;
            }
            if i.key_pressed(Key::D) {
                if let Some(id) = self.inbox.selected_id {
                    self.server.store().delete(id);
                    self.inbox.selected_id = None;
                }
            }
            for (idx, key) in [
                Key::Num1,
                Key::Num2,
                Key::Num3,
                Key::Num4,
                Key::Num5,
                Key::Num6,
            ]
            .iter()
            .enumerate()
            {
                if i.key_pressed(*key) {
                    if let Some(t) = DetailTab::ALL.get(idx).copied() {
                        self.detail.selected_tab = t;
                    }
                }
            }
        });
    }

    fn apply_theme_if_changed(&mut self, ctx: &egui::Context) {
        if self.last_applied_theme != self.settings.theme {
            theme::apply(ctx, self.settings.theme);
            self.last_applied_theme = self.settings.theme;
        }
    }
}

impl eframe::App for MailboxApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.subscription.refresh();
        self.refresh_snapshot();
        self.apply_theme_if_changed(&ctx);

        // Global shortcuts that don't need text input.
        let mut clear_via_shortcut = false;
        let mut settings_via_shortcut = false;
        let mut help_via_shortcut = false;
        toolbar::handle_global_shortcuts(
            &ctx,
            &mut self.paused,
            &mut self.settings.theme,
            &mut self.pending_focus_search,
            &mut clear_via_shortcut,
            &mut help_via_shortcut,
            &mut settings_via_shortcut,
        );
        self.handle_inbox_keys(&ctx);

        // Toolbar.
        let smtp_url = self.smtp_url();
        let mut focus_search = self.pending_focus_search;
        self.pending_focus_search = false;

        let mut tb_out = toolbar::ToolbarOutput::default();
        let server_settings = self.server.settings();
        let relay_label = server_settings.relay.as_ref().and_then(|r| {
            url::Url::parse(&r.url).ok().and_then(|u| {
                u.host_str().map(|h| {
                    if let Some(p) = u.port() {
                        format!("{h}:{p}")
                    } else {
                        h.to_string()
                    }
                })
            })
        });
        let visuals = ctx.global_style().visuals.clone();
        let toolbar_frame = egui::Frame::default()
            .fill(visuals.panel_fill)
            .stroke(egui::Stroke::NONE)
            .inner_margin(egui::Margin::ZERO)
            .outer_margin(egui::Margin::ZERO);
        egui::Panel::top("toolbar")
            .exact_size(56.0)
            .resizable(false)
            .frame(toolbar_frame)
            .show_inside(ui, |ui| {
                tb_out = toolbar::render(
                    ui,
                    ToolbarContext {
                        smtp_url: &smtp_url,
                        message_count: self.list_snapshot.len(),
                        search_query: &mut self.inbox.search_query,
                        paused: &mut self.paused,
                        theme: &mut self.settings.theme,
                        toasts: &mut self.toasts,
                        focus_search,
                        relay_active: server_settings.relay.is_some(),
                        relay_label: relay_label.as_deref(),
                    },
                );
                focus_search = false;
                // Bottom hairline separator.
                let r = ui.max_rect();
                ui.painter().line_segment(
                    [
                        egui::pos2(r.left(), r.bottom() - 0.5),
                        egui::pos2(r.right(), r.bottom() - 0.5),
                    ],
                    egui::Stroke::new(1.0, visuals.widgets.noninteractive.bg_stroke.color),
                );
            });

        if tb_out.clear_clicked || clear_via_shortcut {
            if self.list_snapshot.is_empty() {
                self.toasts.info("Inbox is already empty");
            } else {
                self.confirm_clear = true;
            }
        }
        if tb_out.settings_clicked || settings_via_shortcut {
            self.settings_window.open_with(&self.server.settings());
        }
        if tb_out.help_clicked || help_via_shortcut {
            self.help_window.open = true;
        }
        if tb_out.relay_clicked {
            self.relay_window.open_with(&self.server.settings());
        }

        // Inbox + detail panes.
        let snapshot = self.list_snapshot.clone();
        let paused = self.paused;
        let mut copy_swaks: Option<String> = None;
        let inbox_frame = egui::Frame::default()
            .fill(visuals.panel_fill)
            .stroke(egui::Stroke::NONE)
            .inner_margin(egui::Margin::ZERO);
        egui::Panel::left("inbox")
            .default_size(380.0)
            .min_size(280.0)
            .frame(inbox_frame)
            .show_inside(ui, |ui| {
                let action = inbox::render(
                    ui,
                    &mut self.inbox,
                    &snapshot,
                    InboxRenderContext {
                        paused,
                        smtp_url: &smtp_url,
                        on_copy_swaks: &mut copy_swaks,
                    },
                );
                if let InboxAction::Selected(_) = action {
                    // Inbox::render already updated `inbox.selected_id`.
                }
                // Right hairline separator.
                let r = ui.max_rect();
                ui.painter().line_segment(
                    [
                        egui::pos2(r.right() - 0.5, r.top()),
                        egui::pos2(r.right() - 0.5, r.bottom()),
                    ],
                    egui::Stroke::new(1.0, visuals.widgets.noninteractive.bg_stroke.color),
                );
            });
        if let Some(snippet) = copy_swaks {
            ctx.copy_text(snippet);
            self.toasts.success("Copied swaks command");
        }

        let selected = self
            .inbox
            .selected_id
            .and_then(|id| self.list_snapshot.iter().find(|m| m.id == id).cloned());
        let server = self.server.clone();
        let mut toasts = std::mem::take(&mut self.toasts);
        #[cfg(target_os = "macos")]
        let native_html = self.native_html.as_ref();
        // Detail (central) pane uses the *body* fill so it sits visually
        // below the slightly-elevated toolbar + list panels.
        let detail_frame = egui::Frame::default()
            .fill(visuals.window_fill)
            .stroke(egui::Stroke::NONE)
            .inner_margin(egui::Margin::ZERO);
        egui::CentralPanel::default()
            .frame(detail_frame)
            .show_inside(ui, |ui| {
                let mut dctx = DetailContext {
                    server: &server,
                    toasts: &mut toasts,
                    #[cfg(target_os = "macos")]
                    native_html,
                };
                detail::render(ui, &mut self.detail, selected.as_ref(), &mut dctx);
            });
        self.toasts = toasts;

        // Hide the native HTML view if no message is selected, so it doesn't
        // hang around displaying stale content while the user's on the empty
        // detail placeholder.
        #[cfg(target_os = "macos")]
        if selected.is_none() {
            if let Some(view) = self.native_html.as_ref() {
                view.set_visible(false);
            }
        }

        // Modal-ish windows: rendered after the panels so they float on top.
        settings_window::render(
            &ctx,
            &mut self.settings_window,
            &self.server,
            &mut self.toasts,
        );
        relay_window::render(&ctx, &mut self.relay_window, &self.server, &mut self.toasts);
        help_window::render(&ctx, &mut self.help_window);

        // Confirm-clear modal.
        if self.confirm_clear {
            let mut keep_open = true;
            let mut confirm = false;
            let mut cancel = false;
            let count = self.list_snapshot.len();
            egui::Window::new(egui::RichText::new("Clear inbox").strong())
                .open(&mut keep_open)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(&ctx, |ui| {
                    ui.label(format!(
                        "Discard all {count} captured message{}? This can't be undone.",
                        if count == 1 { "" } else { "s" }
                    ));
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .button(
                                    egui::RichText::new("Clear all")
                                        .color(egui::Color32::from_rgb(248, 113, 113)),
                                )
                                .clicked()
                            {
                                confirm = true;
                            }
                        });
                    });
                });
            if !keep_open || cancel {
                self.confirm_clear = false;
            }
            if confirm {
                self.server.store().clear();
                self.inbox.selected_id = None;
                self.toasts.info("Inbox cleared");
                self.confirm_clear = false;
            }
        }

        // Hide the WKWebView while a modal is up so it doesn't float over the
        // dialog content.
        #[cfg(target_os = "macos")]
        if self.settings_window.open
            || self.relay_window.open
            || self.help_window.open
            || self.confirm_clear
        {
            if let Some(view) = self.native_html.as_ref() {
                view.set_visible(false);
            }
        }

        // Toasts overlay last so they sit above panels and dialogs.
        self.toasts.show(&ctx);

        // Idle-friendly repaint cap. The store-event bridge is the primary
        // wakeup; this is the safety net for any signal we missed.
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
}
