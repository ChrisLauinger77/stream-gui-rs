//! Native dispatch for destinations already validated by authentication or chat.
//!
//! Keep this internal: neither IPC nor provider metadata may supply arbitrary URLs.
pub(crate) fn open(uri: &str) -> std::result::Result<(), ()> {
    #[cfg(target_os = "linux")]
    {
        // GIO honors the desktop's URI handler and owns launcher cleanup. The
        // webbrowser crate's Linux background spawn drops its Child without reaping.
        // A private context lets this blocking worker await D-Bus dispatch without
        // borrowing GTK's main-thread context or waiting for the browser to close.
        gio::glib::MainContext::new()
            .block_on(gio::AppInfo::launch_default_for_uri_future(
                uri,
                None::<&gio::AppLaunchContext>,
            ))
            .map_err(|_| ())
    }
    #[cfg(not(target_os = "linux"))]
    {
        webbrowser::open(uri).map_err(|_| ())
    }
}
