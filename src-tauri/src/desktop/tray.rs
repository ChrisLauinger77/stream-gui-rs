use super::localization as l10n;
use super::*;
use crate::domain::background::DesktopAction;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};

const GLYPH_SIZE: u32 = 64;
#[cfg(any(target_os = "linux", target_os = "windows"))]
const WHITE: [u8; 3] = [255, 255, 255];
const BLACK: [u8; 3] = [0, 0, 0];

/// Outline drawn around a glyph: color, alpha and width in design units.
type Halo = ([u8; 3], u8, f32);

/// Linux indicator hosts show pixmaps untinted on light or dark panels, and the
/// tray may be the only way back to a hidden window. A translucent dark
/// outline keeps the white glyph visible on both.
#[cfg(target_os = "linux")]
const PANEL_HALO: Halo = (BLACK, 180, 1.0);

/// Rasterizes the status-area glyph: the logo's stream lines and play mark in
/// one color with antialiased alpha, optionally outlined. Status areas
/// conventionally show monochrome icons, while windows, bundles and
/// notifications keep the colored application icon. Geometry uses a 32-unit
/// design grid.
fn glyph(size: u32, color: [u8; 3], halo: Option<Halo>) -> Vec<u8> {
    const SAMPLES: u32 = 4;
    let scale = 32.0 / size as f32;
    let sample =
        |pixel: u32, step: u32| (pixel as f32 + (step as f32 + 0.5) / SAMPLES as f32) * scale;
    let (halo_color, halo_alpha, grow) = halo.unwrap_or((color, 0, 0.0));
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let (mut fill, mut outline) = (0u32, 0u32);
            for step in 0..SAMPLES * SAMPLES {
                let (sx, sy) = (sample(x, step % SAMPLES), sample(y, step / SAMPLES));
                if glyph_covers(sx, sy, 0.0) {
                    fill += 1;
                } else if grow > 0.0 && glyph_covers(sx, sy, grow) {
                    outline += 1;
                }
            }
            // Fill and outline samples are disjoint, so their weights add.
            let (fill, outline) = (fill * 255, outline * u32::from(halo_alpha));
            let total = fill + outline;
            let channel = |c: u8, h: u8| match total {
                0 => c,
                _ => ((fill * u32::from(c) + outline * u32::from(h)) / total) as u8,
            };
            rgba.extend_from_slice(&[
                channel(color[0], halo_color[0]),
                channel(color[1], halo_color[1]),
                channel(color[2], halo_color[2]),
                (total / (SAMPLES * SAMPLES)) as u8,
            ]);
        }
    }
    rgba
}

/// Whether a point lies within `grow` units of the glyph. Expanding every
/// stroke radius dilates the shape exactly because the play mark is convex.
fn glyph_covers(x: f32, y: f32, grow: f32) -> bool {
    let near = |(ax, ay): (f32, f32), (bx, by): (f32, f32), radius: f32| {
        let (dx, dy) = (bx - ax, by - ay);
        let t = (((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
        let (ex, ey) = (x - ax - t * dx, y - ay - t * dy);
        ex * ex + ey * ey <= radius * radius
    };
    let lines = [
        ((6.0, 9.5), (13.0, 9.5)),
        ((3.0, 16.0), (13.0, 16.0)),
        ((7.0, 22.5), (13.0, 22.5)),
    ];
    if lines.iter().any(|&(a, b)| near(a, b, 1.5 + grow)) {
        return true;
    }
    // Play mark with one-unit rounded corners.
    let (top, bottom, tip): ((f32, f32), (f32, f32), (f32, f32)) =
        ((17.5, 7.5), (17.5, 24.5), (29.0, 16.0));
    let rise = (x - top.0) * (tip.1 - top.1) / (tip.0 - top.0);
    let inside = x >= top.0 && y >= top.1 + rise && y <= bottom.1 - rise;
    let edge = 1.0 + grow;
    inside || near(top, bottom, edge) || near(top, tip, edge) || near(bottom, tip, edge)
}

fn glyph_image(color: [u8; 3], halo: Option<Halo>) -> Image<'static> {
    Image::new_owned(glyph(GLYPH_SIZE, color, halo), GLYPH_SIZE, GLYPH_SIZE)
}

/// Windows has no template icons; follow the taskbar's own light/dark mode.
/// A missing value means a pre-1903 dark taskbar.
#[cfg(target_os = "windows")]
fn light_taskbar() -> bool {
    use ::windows::{
        Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW},
        core::w,
    };
    let mut value = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: fixed NUL-terminated names; RRF_RT_REG_DWORD restricts the
    // result to a DWORD written into this exactly sized buffer.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
            w!("SystemUsesLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some((&raw mut value).cast()),
            Some(&raw mut size),
        )
    };
    status.is_ok() && value != 0
}

#[cfg(target_os = "windows")]
fn taskbar_glyph(light: bool) -> Image<'static> {
    glyph_image(if light { BLACK } else { WHITE }, None)
}

#[cfg(target_os = "linux")]
fn indicator_available() -> bool {
    [
        c"libayatana-appindicator3.so.1",
        c"libappindicator3.so.1",
        c"libayatana-appindicator3.so",
        c"libappindicator3.so",
    ]
    .iter()
    .any(|name| {
        // SAFETY: these are fixed system-library names with the ABI used by
        // Tauri. No symbols are invoked, and the successful handle is balanced.
        let handle = unsafe { libc::dlopen(name.as_ptr(), libc::RTLD_LAZY | libc::RTLD_LOCAL) };
        if handle.is_null() {
            false
        } else {
            unsafe {
                libc::dlclose(handle);
            }
            true
        }
    })
}
pub(super) fn install(app: &tauri::AppHandle) -> tauri::Result<bool> {
    #[cfg(target_os = "linux")]
    if !indicator_available() {
        return Ok(false);
    }
    let locale = l10n::current(app);
    let show = MenuItem::with_id(
        app,
        "show",
        l10n::text(locale, "native.show"),
        true,
        None::<&str>,
    )?;
    let hide = MenuItem::with_id(
        app,
        "hide",
        l10n::text(locale, "native.hide"),
        true,
        None::<&str>,
    )?;
    let pause = MenuItem::with_id(
        app,
        "monitor",
        l10n::text(locale, "native.pause"),
        false,
        None::<&str>,
    )?;
    let watching = MenuItem::with_id(
        app,
        "watching",
        l10n::counted(locale, "native.watching", 0),
        true,
        None::<&str>,
    )?;
    let about = MenuItem::with_id(
        app,
        "about",
        l10n::text(locale, "native.about"),
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(
        app,
        "quit",
        l10n::text(locale, "native.quit"),
        true,
        None::<&str>,
    )?;
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &hide,
            &pause,
            &watching,
            &PredefinedMenuItem::separator(app)?,
            &about,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;
    let builder = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .tooltip("Stream GUI RS")
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_window(app),
            "hide" => hide_window(app),
            "monitor" => {
                let services = app.state::<Arc<Services>>();
                services.monitor.pause(!services.monitor.snapshot().paused);
            }
            "watching" => {
                app.state::<Arc<notifications::Notifications>>()
                    .shared
                    .state
                    .lock()
                    .expect("native notification state poisoned")
                    .action = Some(DesktopAction::Watching {
                    id: uuid::Uuid::new_v4().to_string(),
                });
                show_window(app);
            }
            "about" => show_about(app),
            "quit" => begin_shutdown(app),
            _ => {}
        });
    // macOS tints a template image for the menu bar appearance itself.
    #[cfg(target_os = "macos")]
    let builder = builder
        .icon(glyph_image(BLACK, None))
        .icon_as_template(true);
    #[cfg(target_os = "windows")]
    let mut light = light_taskbar();
    #[cfg(target_os = "windows")]
    let builder = builder.icon(taskbar_glyph(light));
    // Linux indicator hosts cannot recolor pixmaps; outline for any panel color.
    #[cfg(target_os = "linux")]
    let builder = builder.icon(glyph_image(WHITE, Some(PANEL_HALO)));
    let tray = builder.build(app)?;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut previous = None;
        loop {
            if app
                .state::<Arc<Lifecycle>>()
                .stopping
                .load(Ordering::SeqCst)
            {
                break;
            }
            #[cfg(target_os = "windows")]
            if let Ok(current) = tauri::async_runtime::spawn_blocking(light_taskbar).await
                && current != light
            {
                let _ = tray.set_icon(Some(taskbar_glyph(current)));
                light = current;
            }
            let services = app.state::<Arc<Services>>();
            let status = services.monitor.snapshot();
            let settings = services.settings.snapshot();
            let enabled = settings.background.monitoring_enabled;
            let locale = l10n::selected(settings.ui_language);
            let count = services
                .sessions
                .sessions()
                .await
                .iter()
                .filter(|s| {
                    s.restarting
                        || matches!(
                            s.phase,
                            crate::streamlink::SessionPhase::Starting
                                | crate::streamlink::SessionPhase::Running
                                | crate::streamlink::SessionPhase::Stopping
                        )
                })
                .count();
            let host = app
                .state::<Arc<notifications::Notifications>>()
                .shared
                .state
                .lock()
                .expect("native notification state poisoned")
                .tray_host;
            let current = (status.phase, status.paused, enabled, count, host, locale);
            if previous != Some(current) {
                let _ = show.set_text(l10n::text(locale, "native.show"));
                let _ = hide.set_text(l10n::text(locale, "native.hide"));
                let _ = about.set_text(l10n::text(locale, "native.about"));
                let _ = pause.set_text(if status.paused {
                    l10n::text(locale, "native.resume")
                } else {
                    l10n::text(locale, "native.pause")
                });
                let _ = pause.set_enabled(enabled);
                let _ = watching.set_text(l10n::counted(locale, "native.watching", count));
                let _ = quit.set_text(if count == 0 {
                    l10n::text(locale, "native.quit").into()
                } else {
                    l10n::counted(
                        locale,
                        if count == 1 {
                            "native.quitStreams.one"
                        } else {
                            "native.quitStreams.other"
                        },
                        count,
                    )
                });
                let _ = hide.set_enabled(host);
                let _ = tray.set_tooltip(Some(format!(
                    "Stream GUI RS · {} · {}",
                    l10n::text(locale, "native.monitoring")
                        .replace("{phase}", l10n::phase(locale, status.phase)),
                    l10n::counted(locale, "native.watching", count)
                )));
                // An indicator host disappearing must not strand a hidden app.
                if !host
                    && app
                        .get_webview_window("main")
                        .is_some_and(|window| window.is_visible().is_ok_and(|visible| !visible))
                {
                    show_window(&app);
                }
                previous = Some(current);
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_glyph_is_single_color_with_separated_marks() {
        let rgba = glyph(32, [10, 20, 30], None);
        assert_eq!(rgba.len(), 32 * 32 * 4);
        assert!(rgba.chunks(4).all(|pixel| pixel[..3] == [10, 20, 30]));
        let alpha = |x: usize, y: usize| rgba[(y * 32 + x) * 4 + 3];
        // Stream line, play mark, and the transparent gap and margin around them.
        assert_eq!(alpha(8, 16), 255);
        assert_eq!(alpha(21, 16), 255);
        assert_eq!(alpha(15, 16), 0);
        assert_eq!(alpha(0, 0), 0);
        assert!(rgba.chunks(4).any(|pixel| pixel[3] > 0 && pixel[3] < 255));
    }

    #[test]
    fn tray_glyph_halo_outlines_marks_for_any_panel_color() {
        let rgba = glyph(64, [250, 250, 250], Some(([0, 0, 0], 180, 1.0)));
        let pixel = |x: usize, y: usize| &rgba[(y * 64 + x) * 4..][..4];
        // Fill keeps its color; the band just outside a stream line and the
        // gap before the play mark become the translucent outline.
        assert_eq!(pixel(16, 32), [250, 250, 250, 255]);
        assert_eq!(pixel(16, 35), [0, 0, 0, 180]);
        assert_eq!(pixel(30, 32), [0, 0, 0, 180]);
        assert_eq!(pixel(0, 0)[3], 0);
    }
}
