#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).unwrap();
    /*slint::platform::set_platform(Box::new(
        slint::internal::backend::android_activity::i_slint_backend_android_activity::AndroidPlatform::new(app),
    ))
    .unwrap();*/
    client::main().unwrap();
}
