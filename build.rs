fn main() {
    // Windows exe icon + version metadata; windres handles the gnu target.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/icon.ico")
            .set("ProductName", "Slipstream")
            .set("FileDescription", "Slipstream arcade launcher")
            .compile()
            .expect("embedding Windows resources");
    }
}
