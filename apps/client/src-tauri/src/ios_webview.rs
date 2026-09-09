//! Gives the iOS WebView the whole screen.
//!
//! Tauri sizes a webview to the window's inner size, and on iOS `tao` reports
//! that as the *safe area*: on an iPhone 17 Pro the WebView comes out 402x778
//! on a 402x874 screen. It still sits at the top of the screen, so the missing
//! 96pt all lands at the bottom as a band of native background.
//!
//! Meanwhile `env(safe-area-inset-*)` keeps reporting the device's real insets
//! (62pt and 34pt here), because WebKit reads them from the window rather than
//! from the WebView's frame. `src/index.css` pads the app frame with them --
//! which is correct, and is what the web and Android builds need -- so the
//! bottom inset is counted twice and the app stops 130pt short of the bottom
//! of the screen.
//!
//! Stretching the WebView over its superview fixes the cause rather than the
//! symptom: the superview is the window-sized view `tao` maintains, so the
//! insets then describe the view they are applied to.

use objc2::msg_send;
use objc2::rc::Retained;
use objc2_ui_kit::{
    UIScrollView, UIScrollViewContentInsetAdjustmentBehavior, UIView,
    UIViewAutoresizing,
};
use tauri::WebviewWindow;

/// Lays the window's WebView over the whole screen, once it exists.
///
/// Failures here cost 15% of the screen, not correctness, so every one of them
/// is logged and stepped over rather than propagated.
pub fn stretch_to_window(window: &WebviewWindow) {
    if let Err(error) = window.with_webview(|webview| {
        // SAFETY: Tauri hands the closure the live `WKWebView` on the main
        // thread, which is where UIKit requires this work to happen.
        unsafe { stretch_to_superview(webview.inner()) }
    }) {
        log::error!(
            "could not reach the iOS webview to resize it: {error}"
        );
    }
}

/// # Safety
///
/// `webview` must be a live `WKWebView` that already has a superview, and the
/// caller must be on the main thread.
unsafe fn stretch_to_superview(webview: *mut std::ffi::c_void) {
    if webview.is_null() {
        return;
    }

    let webview = &*webview.cast::<UIView>();
    let Some(parent) = webview.superview() else {
        log::warn!(
            "iOS webview has no superview yet; leaving its frame alone"
        );
        return;
    };

    // The autoresizing mask, not constraints: `tao` resets its own view to the
    // window bounds whenever the window lays out, and a flexible mask carries
    // that straight through to the WebView. Rotation needs nothing else.
    webview.setFrame(parent.bounds());
    webview.setAutoresizingMask(
        UIViewAutoresizing::FlexibleWidth
            | UIViewAutoresizing::FlexibleHeight,
    );

    // With the frame right, an automatic content inset would put the same
    // margin back as scroll offset instead of as a frame. `viewport-fit=cover`
    // plus `env(safe-area-inset-*)` is how this app wants to handle the notch,
    // and that only works if UIKit keeps out of it.
    let scroll_view: Option<Retained<UIScrollView>> =
        msg_send![webview, scrollView];
    match scroll_view {
        Some(scroll_view) => scroll_view.setContentInsetAdjustmentBehavior(
            UIScrollViewContentInsetAdjustmentBehavior::Never,
        ),
        None => log::warn!("iOS webview reported no scroll view"),
    }
}
