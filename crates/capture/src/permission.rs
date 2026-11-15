//! OS screen-capture permission (macOS Screen Recording; always granted on Windows).

#[cfg(target_os = "macos")]
mod imp {
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGPreflightScreenCaptureAccess() -> bool;
        fn CGRequestScreenCaptureAccess() -> bool;
    }

    const SETTINGS_URL: &str =
        "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture";

    pub fn granted() -> bool {
        // SAFETY: no arguments, no side effects.
        unsafe { CGPreflightScreenCaptureAccess() }
    }

    pub fn request() -> bool {
        // SAFETY: no arguments; shows the system prompt at most once per app.
        unsafe { CGRequestScreenCaptureAccess() }
    }

    pub fn open_settings() -> std::io::Result<()> {
        std::process::Command::new("open").arg(SETTINGS_URL).spawn().map(|_| ())
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    pub fn granted() -> bool {
        true
    }

    pub fn request() -> bool {
        true
    }

    pub fn open_settings() -> std::io::Result<()> {
        Ok(())
    }
}

/// True when the process may capture other apps' windows.
pub fn granted() -> bool {
    imp::granted()
}

/// Asks the OS for permission; on macOS the prompt only appears the first time.
pub fn request() -> bool {
    imp::request()
}

/// Opens the OS settings page where the permission is toggled.
pub fn open_settings() -> std::io::Result<()> {
    imp::open_settings()
}
