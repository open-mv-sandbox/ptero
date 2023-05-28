mod surface;
mod viewer;

use tracing::Level;
use tracing_wasm::WASMLayerConfigBuilder;
pub use viewer::Viewer;

/// Initialize global hooks that may not yet be initialized.
fn init_hooks() {
    console_error_panic_hook::set_once();
    let config = WASMLayerConfigBuilder::new()
        .set_max_level(Level::INFO)
        .build();
    tracing_wasm::set_as_global_default_with_config(config);
}
