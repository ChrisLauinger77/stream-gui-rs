//! Keep the standard AppKit About panel, including its icon and native link handling.
use crate::build_info;
use objc2::{AnyThread, MainThreadMarker, rc::Retained, runtime::AnyObject};
use objc2_app_kit::{
    NSAboutPanelOptionApplicationName, NSAboutPanelOptionApplicationVersion,
    NSAboutPanelOptionCredits, NSAboutPanelOptionVersion, NSApplication, NSColor,
    NSForegroundColorAttributeName, NSLinkAttributeName, NSUnderlineStyle,
    NSUnderlineStyleAttributeName,
};
use objc2_foundation::{NSAttributedString, NSDictionary, NSNumber, NSString, NSURL};
use tauri::{
    AppHandle, Runtime,
    menu::{Menu, MenuItem},
};

pub(super) const MENU_ID: &str = "app-about";

pub(super) fn menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let menu = Menu::default(app)?;
    let items = menu.items()?;
    // Tauri's default macOS menu starts with the application submenu and About.
    // Replace only that item, preserving Services, editing shortcuts and Quit.
    let application = items
        .first()
        .and_then(|item| item.as_submenu())
        .ok_or_else(|| std::io::Error::other("Native application menu is missing."))?;
    let about = MenuItem::with_id(
        app,
        MENU_ID,
        format!("About {}", build_info::NAME),
        true,
        None::<&str>,
    )?;
    application.remove_at(0)?;
    application.prepend(&about)?;
    Ok(menu)
}

pub(super) fn show<R: Runtime>(app: &AppHandle<R>) {
    let info = build_info::snapshot();
    let _ = app.run_on_main_thread(move || {
        if let Some(main) = MainThreadMarker::new() {
            let options = options(&info.name, &info.version, &info.commit);
            // SAFETY: all option keys have their documented AppKit value types;
            // the panel is opened on the main thread and retains its own values.
            unsafe {
                NSApplication::sharedApplication(main)
                    .orderFrontStandardAboutPanelWithOptions(&options);
            }
        }
    });
}

fn options(name: &str, version: &str, commit: &str) -> Retained<NSDictionary<NSString, AnyObject>> {
    let name = NSString::from_str(name);
    let version = NSString::from_str(version);
    let commit = NSString::from_str(commit);
    let label = NSString::from_str("Github Repository");
    let repository = NSURL::URLWithString(&NSString::from_str(build_info::REPOSITORY))
        .expect("The compiled repository URL is valid");
    let color = NSColor::linkColor();
    let underline = NSNumber::new_isize(NSUnderlineStyle::Single.0);
    // AppKit prefers NSURL link values. Explicit native link styling keeps the
    // credits visibly actionable instead of relying on the panel's text defaults.
    // SAFETY: attribute keys receive their documented NSURL/NSColor/NSNumber types.
    // Text and destination are fixed project metadata, never untrusted input.
    let credits = unsafe {
        let attributes = NSDictionary::from_slices(
            &[
                NSLinkAttributeName,
                NSForegroundColorAttributeName,
                NSUnderlineStyleAttributeName,
            ],
            &[&*repository as &AnyObject, &*color, &*underline],
        );
        NSAttributedString::initWithString_attributes(
            NSAttributedString::alloc(),
            &label,
            Some(&attributes),
        )
    };
    // AppKit formats ApplicationVersion and Version as "Version <version> (<build>)".
    // The native credits area renders the link; no custom window or URL IPC exists.
    // SAFETY: these exported AppKit keys receive NSString or NSAttributedString
    // values as documented, and NSDictionary retains them past this function.
    unsafe {
        NSDictionary::from_slices(
            &[
                NSAboutPanelOptionApplicationName,
                NSAboutPanelOptionApplicationVersion,
                NSAboutPanelOptionVersion,
                NSAboutPanelOptionCredits,
            ],
            &[&*name as &AnyObject, &*version, &*commit, &*credits],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_options_separate_version_and_commit_and_include_repository_link() {
        let options = options("Stream GUI RS", "1.2.3", "a1b2c3d");
        // SAFETY: known option/attribute keys, indices within the fixed label,
        // and a null effective-range output is supported.
        unsafe {
            assert_eq!(
                options
                    .objectForKey(NSAboutPanelOptionApplicationVersion)
                    .unwrap()
                    .downcast_ref::<NSString>()
                    .unwrap()
                    .to_string(),
                "1.2.3"
            );
            assert_eq!(
                options
                    .objectForKey(NSAboutPanelOptionVersion)
                    .unwrap()
                    .downcast_ref::<NSString>()
                    .unwrap()
                    .to_string(),
                "a1b2c3d"
            );
            let credits = options.objectForKey(NSAboutPanelOptionCredits).unwrap();
            let credits = credits.downcast_ref::<NSAttributedString>().unwrap();
            assert_eq!(credits.string().to_string(), "Github Repository");
            for index in 0..credits.length() {
                let attribute = |key| {
                    credits
                        .attribute_atIndex_effectiveRange(key, index, std::ptr::null_mut())
                        .unwrap()
                };
                assert_eq!(
                    attribute(NSLinkAttributeName)
                        .downcast_ref::<NSURL>()
                        .unwrap()
                        .absoluteString()
                        .unwrap()
                        .to_string(),
                    build_info::REPOSITORY
                );
                assert!(
                    attribute(NSForegroundColorAttributeName)
                        .downcast_ref::<NSColor>()
                        .is_some()
                );
                assert_eq!(
                    attribute(NSUnderlineStyleAttributeName)
                        .downcast_ref::<NSNumber>()
                        .unwrap()
                        .as_isize(),
                    NSUnderlineStyle::Single.0
                );
            }
        }
    }
}
