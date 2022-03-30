// TODO: Add Browser Support
// TODO: maybe consider a bridge to make the launcher work on mobile 🤔

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let app = clowncher::App::default();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
