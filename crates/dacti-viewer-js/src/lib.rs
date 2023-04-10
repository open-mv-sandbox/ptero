mod surface;
mod viewer;

pub use viewer::Viewer;

/// Initialize global hooks that may not yet be initialized.
fn init_hooks() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
}
