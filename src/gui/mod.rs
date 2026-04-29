//! egui front-end. Owns view state, draws the toolbar / inbox / detail
//! panes, and forwards user actions to the [`ServerHandle`].
//!
//! Phase 4 wires the shell + inbox + a placeholder detail pane. The real
//! tabs (Phase 5), native HTML preview (Phase 6), and Preferences /
//! Relay / Help windows (Phase 7) build on this skeleton.

pub mod detail;
pub mod inbox;
pub mod repaint;
pub mod theme;
pub mod toasts;
pub mod toolbar;

use std::sync::Arc;

use egui::Key;
use uuid::Uuid;

use crate::message::Message;
use crate::server::ServerHandle;
use crate::settings::PersistentSettings;

use self::detail::{DetailState, DetailTab};
use self::inbox::{InboxAction, InboxState};
use self::repaint::StoreSubscription;
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
}

impl MailboxApp {
    pub fn new(server: Arc<ServerHandle>, egui_ctx: egui::Context) -> Self {
        let settings = server.settings();
        theme::apply(&egui_ctx, settings.theme);
        let subscription = StoreSubscription::new(
            server.clone(),
            tokio::runtime::Handle::current(),
            egui_ctx.clone(),
        );
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
                self.inbox.selected_id = visible.get(next).copied();
            }
            if i.key_pressed(Key::K) || i.key_pressed(Key::ArrowUp) {
                let prev = current.map(|p| p.saturating_sub(1)).unwrap_or(0);
                self.inbox.selected_id = visible.get(prev).copied();
            }
            if i.key_pressed(Key::G) {
                if i.modifiers.shift {
                    self.inbox.selected_id = visible.last().copied();
                } else {
                    self.inbox.selected_id = visible.first().copied();
                }
            }
            if i.key_pressed(Key::D) {
                if let Some(id) = self.inbox.selected_id {
                    self.server.store().delete(id);
                    self.inbox.selected_id = None;
                }
            }
            for (idx, key) in [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6]
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
        egui::Panel::top("toolbar")
            .exact_size(48.0)
            .resizable(false)
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
                    },
                );
                focus_search = false;
            });

        if tb_out.clear_clicked || clear_via_shortcut {
            self.server.store().clear();
            self.inbox.selected_id = None;
            self.toasts.info("Inbox cleared");
        }
        if tb_out.settings_clicked || settings_via_shortcut {
            self.toasts
                .info("Preferences window will land in a later step");
        }
        if tb_out.help_clicked || help_via_shortcut {
            self.toasts.info("Help dialog will land in a later step");
        }
        if tb_out.relay_clicked {
            self.toasts.info("Relay window will land in a later step");
        }

        // Inbox + detail panes.
        let snapshot = self.list_snapshot.clone();
        let paused = self.paused;
        egui::Panel::left("inbox")
            .default_size(380.0)
            .min_size(280.0)
            .show_inside(ui, |ui| {
                let action = inbox::render(ui, &mut self.inbox, &snapshot, paused);
                if let InboxAction::Selected(_) = action {
                    // Inbox::render already updated `inbox.selected_id`.
                }
            });

        let selected = self.inbox.selected_id.and_then(|id| {
            self.list_snapshot
                .iter()
                .find(|m| m.id == id)
                .cloned()
        });
        egui::CentralPanel::default().show_inside(ui, |ui| {
            detail::render(ui, &mut self.detail, selected.as_ref());
        });

        // Toasts overlay last so they sit above panels.
        self.toasts.show(&ctx);

        // Idle-friendly repaint cap. The store-event bridge is the primary
        // wakeup; this is the safety net for any signal we missed.
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
}
