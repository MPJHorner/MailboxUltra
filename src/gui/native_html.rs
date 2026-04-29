//! Native macOS WKWebView, embedded as a sibling NSView of the eframe
//! window's content view. Used by the HTML tab to render captured email
//! HTML with the system WebKit engine — pixel-perfect, no Chromium, no
//! HTTP server.
//!
//! Locked-down configuration:
//! - JavaScript disabled (`WKPreferences::setJavaScriptEnabled(false)`).
//! - No base URL when loading HTML, so relative URLs go nowhere.
//! - A custom `WKNavigationDelegate` cancels every link click and shells
//!   out to the user's default browser via `open::that`.
//!
//! This file is `cfg(target_os = "macos")` only. The unix path on this
//! platform should always succeed (every Mac has WebKit).

#![cfg(target_os = "macos")]

use std::cell::Cell;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, MainThreadOnly};
use objc2_app_kit::{NSView, NSWindow, NSWindowOrderingMode};
use objc2_core_foundation::CGFloat;
use objc2_foundation::{
    MainThreadMarker, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};
use objc2_web_kit::{
    WKNavigationAction, WKNavigationActionPolicy, WKNavigationDelegate, WKNavigationType,
    WKPreferences, WKWebView, WKWebViewConfiguration,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Owns one `WKWebView` for the lifetime of the app. The view is added to
/// the eframe window's contentView the first time `attach()` succeeds.
pub struct NativeHtmlView {
    web: Retained<WKWebView>,
    /// Retained handle to the contentView the WKWebView is a subview of.
    /// We keep it so we can ask its current frame for the y-flip on every
    /// `set_frame`, instead of relying on egui's idea of the window size
    /// (which excludes panels and gives the wrong basis for AppKit coords).
    parent: Retained<NSView>,
    delegate: Retained<MBUNavDelegate>,
    last_loaded: Cell<Option<uuid::Uuid>>,
    last_visible: Cell<bool>,
}

impl NativeHtmlView {
    /// Build the WKWebView and add it to the eframe window's contentView.
    /// Returns `None` if we couldn't extract an `NSView*` from the window
    /// handle (e.g. we're not on the main thread, or the platform changed).
    pub fn attach(window: &impl HasWindowHandle) -> Option<Self> {
        let mtm = MainThreadMarker::new()?;

        // Extract the AppKit NSView pointer from raw-window-handle.
        let handle = window.window_handle().ok()?;
        let ns_view_ptr = match handle.as_raw() {
            RawWindowHandle::AppKit(h) => h.ns_view,
            _ => return None,
        };

        // SAFETY: raw-window-handle guarantees ns_view points to a valid
        // NSView for the lifetime of the WindowHandle. We only dereference
        // it here on the main thread (mtm proof) to walk to its window's
        // contentView, and we hold no Rust references across re-entry.
        let parent_view: Retained<NSView> = unsafe {
            let ns_view: &NSView = ns_view_ptr.cast::<NSView>().as_ref();
            let window: Retained<NSWindow> = ns_view.window()?;
            window.contentView()?
        };

        // Configuration: disable JS. The setJavaScriptEnabled method is
        // deprecated by Apple in favour of per-navigation preferences, but
        // the legacy API still works and is the simplest way to lock down
        // the whole web view; we don't render anything that needs JS.
        let config = unsafe { WKWebViewConfiguration::new(mtm) };
        let prefs: Retained<WKPreferences> = unsafe { config.preferences() };
        #[allow(deprecated)]
        unsafe {
            prefs.setJavaScriptEnabled(false);
        }
        unsafe { config.setPreferences(&prefs) };

        // Create the WKWebView.
        let zero_rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(10.0, 10.0));
        let web: Retained<WKWebView> = unsafe {
            WKWebView::initWithFrame_configuration(WKWebView::alloc(mtm), zero_rect, &config)
        };
        web.setHidden(true);

        // Navigation delegate: cancel link clicks, shell to default browser.
        let delegate = MBUNavDelegate::new(mtm);
        unsafe {
            web.setNavigationDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        }

        // Add as the topmost subview of the contentView so it sits above
        // the GL drawing.
        parent_view.addSubview_positioned_relativeTo(&web, NSWindowOrderingMode::Above, None);

        Some(Self {
            web,
            parent: parent_view,
            delegate,
            last_loaded: Cell::new(None),
            last_visible: Cell::new(false),
        })
    }

    /// Show / hide the WKWebView. Called every frame the egui app runs;
    /// the egui repaint cost is negligible since we only ask AppKit when
    /// the visibility actually changes.
    pub fn set_visible(&self, visible: bool) {
        if self.last_visible.get() == visible {
            return;
        }
        self.last_visible.set(visible);
        self.web.setHidden(!visible);
    }

    /// Reposition the web view to overlap the egui rect that was reserved
    /// for the HTML preview. The egui rect is in top-left points relative
    /// to the eframe drawing surface; the WKWebView's frame is in
    /// bottom-left points relative to the parent NSView (the NSWindow's
    /// contentView). We ask the parent for its actual height every call
    /// because the window is resizable and egui's `content_rect` excludes
    /// drawn panels — which would shift the WKWebView up by the toolbar
    /// height and let it cover the tabs.
    pub fn set_frame(&self, rect: egui::Rect) {
        let parent_h: CGFloat = self.parent.frame().size.height;
        let x = rect.left() as CGFloat;
        let w = rect.width() as CGFloat;
        let h = rect.height() as CGFloat;
        let y = parent_h - rect.bottom() as CGFloat;
        let origin = NSPoint::new(x, y);
        let size = NSSize::new(w.max(0.0), h.max(0.0));
        self.web.setFrame(NSRect::new(origin, size));
    }

    /// Replace the WKWebView's contents. No-op if the same message id was
    /// loaded last time (each frame re-calls into here, but we only touch
    /// WebKit when the user actually selects a different message).
    pub fn load(&self, id: uuid::Uuid, html: &str) {
        if self.last_loaded.get() == Some(id) {
            return;
        }
        self.last_loaded.set(Some(id));
        let body = NSString::from_str(html);
        unsafe {
            // Pass `nil` as the base URL so relative href / src paths in
            // the captured email can't trigger network loads.
            let _: Option<Retained<objc2_web_kit::WKNavigation>> =
                self.web.loadHTMLString_baseURL(&body, None);
        }
    }

    /// Empty the WKWebView. Called when the user deselects a message or
    /// navigates away from the HTML tab.
    pub fn clear(&self) {
        if self.last_loaded.get().is_none() {
            return;
        }
        self.last_loaded.set(None);
        let blank = NSString::from_str("");
        unsafe {
            let _: Option<Retained<objc2_web_kit::WKNavigation>> =
                self.web.loadHTMLString_baseURL(&blank, None);
        }
    }
}

impl Drop for NativeHtmlView {
    fn drop(&mut self) {
        // Take the view out of the parent so AppKit doesn't keep referencing
        // freed memory. The Retained<WKWebView> drop after this releases the
        // last ref.
        self.web.removeFromSuperview();
        // Touch the delegate so clippy doesn't warn about it being unused;
        // the WKWebView holds a weak ref to it for navigation policy
        // callbacks.
        let _ = &self.delegate;
    }
}

// ---------------------------------------------------------------------------
// Custom navigation delegate.
// ---------------------------------------------------------------------------

define_class!(
    /// Cancels link clicks and shells out to the user's default browser
    /// via `open::that`. All other navigation policy decisions (e.g. the
    /// initial loadHTMLString) are allowed.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[name = "MBUNavDelegate"]
    #[ivars = ()]
    struct MBUNavDelegate;

    unsafe impl NSObjectProtocol for MBUNavDelegate {}

    unsafe impl WKNavigationDelegate for MBUNavDelegate {
        #[unsafe(method(webView:decidePolicyForNavigationAction:decisionHandler:))]
        fn decide_policy(
            &self,
            _webview: &WKWebView,
            navigation_action: &WKNavigationAction,
            decision_handler: &block2::DynBlock<dyn Fn(WKNavigationActionPolicy)>,
        ) {
            let nav_type = unsafe { navigation_action.navigationType() };
            if nav_type == WKNavigationType::LinkActivated {
                let request = unsafe { navigation_action.request() };
                if let Some(url) = request.URL() {
                    if let Some(s) = url.absoluteString() {
                        let _ = open::that(s.to_string());
                    }
                }
                decision_handler.call((WKNavigationActionPolicy::Cancel,));
            } else {
                decision_handler.call((WKNavigationActionPolicy::Allow,));
            }
        }
    }
);

impl MBUNavDelegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
