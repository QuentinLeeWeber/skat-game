mod app_main;
mod conversions;
mod networking;

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).unwrap();
    app_main::main().unwrap();
}

pub fn start_app() -> Result<(), slint::PlatformError> {
    app_main::main()
}
