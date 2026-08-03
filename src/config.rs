pub const APP_ID: Option<&str> = option_env!("APP_ID");
pub const GETTEXT_PACKAGE: Option<&str> = option_env!("GETTEXT_PACKAGE");
pub const LOCALEDIR: Option<&str> = option_env!("LOCALEDIR");
pub const RESOURCES_FILE: Option<&str> = option_env!("RESOURCES_FILE");
pub const PKGDATADIR: Option<&str> = option_env!("PKGDATADIR");
pub const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");
pub const PROFILE: Option<&str> = option_env!("PROFILE");
pub const AUTHORS: Option<&str> = option_env!("CARGO_PKG_AUTHORS");

pub fn app_id() -> &'static str {
    APP_ID.expect("APP_ID env var not set at compile time")
}

pub fn gettext_package() -> &'static str {
    GETTEXT_PACKAGE.expect("GETTEXT_PACKAGE env var not set at compile time")
}

pub fn localedir() -> &'static str {
    LOCALEDIR.expect("LOCALEDIR env var not set at compile time")
}

pub fn resources_file() -> &'static str {
    RESOURCES_FILE.expect("RESOURCES_FILE env var not set at compile time")
}

pub fn pkgdatadir() -> &'static str {
    PKGDATADIR.expect("PKGDATADIR env var not set at compile time")
}

pub fn version() -> &'static str {
    VERSION.expect("VERSION env var not set at compile time")
}

pub fn profile() -> &'static str {
    PROFILE.expect("PROFILE env var not set at compile time")
}

pub fn authors() -> Vec<&'static str> {
    AUTHORS
        .expect("AUTHORS env var not set at compile time")
        .split(':')
        .collect()
}