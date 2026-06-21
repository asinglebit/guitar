#[test]
fn library_exports_app_and_version() {
    let _app = guitar::App::default();
    let expected = option_env!("GUITAR_BUILD_OVERWRITE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"));

    assert_eq!(guitar::VERSION, expected);
}
