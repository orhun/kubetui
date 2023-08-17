use anyhow::Result;

use kubetui::{app::App, disable_raw_mode, signal::setup_signal_handler};

use std::panic;

fn setup_panic_hook() {
    let default_hook = panic::take_hook();

    panic::set_hook(Box::new(move |info| {
        disable_raw_mode!();

        eprintln!("\x1b[31mPanic! disable raw mode\x1b[39m");

        default_hook(info);
    }));
}

fn main() -> Result<()> {
    setup_signal_handler();

    setup_panic_hook();

    let app = App::init()?;

    app.run()?;

    Ok(())
}
