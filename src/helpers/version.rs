pub const VERSION: &str = match option_env!("GUITAR_BUILD_OVERWRITE_VERSION") {
    Some(version) => version,
    None => env!("CARGO_PKG_VERSION"),
};
