use anyhow::Result;

use kubetui::{app::App, disable_raw_mode, signal::signal_handler};

use std::panic;

fn main() -> Result<()> {
    signal_handler();

    let default_hook = panic::take_hook();

    panic::set_hook(Box::new(move |info| {
        disable_raw_mode!();

        eprintln!("\x1b[31mPanic! disable raw mode\x1b[39m");

        default_hook(info);
    }));

    let app = App::init()?;

    app.run()?;

    Ok(())
}
