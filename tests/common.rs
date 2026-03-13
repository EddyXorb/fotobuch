use std::sync::Once;

#[allow(dead_code)]
static INIT: Once = Once::new();

/// Initialize tracing for tests (called only once per test run)
/// Put this at the beginning of the test you want to debug.
#[allow(dead_code)]
pub fn init_tests() {
    INIT.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_max_level(tracing::Level::DEBUG)
            .try_init();
    });
}
