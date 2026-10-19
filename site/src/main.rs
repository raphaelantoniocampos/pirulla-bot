//! The website of Pirulla Bot, run in the browser as WebAssembly.

mod app;
mod chart;
mod plots;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
