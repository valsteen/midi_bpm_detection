use errors::{Result, initialize_logging, initialize_panic_handler};
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> Result<()> {
    initialize_logging()?;
    initialize_panic_handler(|| {})?;
    Ok(())
}
