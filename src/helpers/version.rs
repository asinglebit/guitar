// pub const VERSION: &str = option_env!("GUITAR_BUILD_OVERWRITE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION")); // require unstable for now
pub const VERSION: &str = match option_env!("GUITAR_BUILD_OVERWRITE_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};
