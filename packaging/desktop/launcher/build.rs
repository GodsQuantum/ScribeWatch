#[cfg(windows)]
fn main() {
    winresource::WindowsResource::new()
        .set_icon("../assets/app.ico")
        .compile()
        .expect("ScribeWatch Windows .exe icon resource");
}
#[cfg(not(windows))]
fn main() {}
