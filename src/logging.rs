/// Initialise panic hooks and tracing backends depending on the platform.
pub fn init() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        tracing_wasm::set_as_global_default();
        tracing::info!(target: "kingshot", "Tracing initialised for wasm target");
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        if tracing::dispatcher::has_been_set() {
            return;
        }

        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_target(false)
            .without_time()
            .init();

        tracing::info!(target: "kingshot", "Tracing initialised for native target");
    }
}
