use anyhow::Result;
use tui_app::App;

fn main() -> Result<()> {
    let mut terminal = tui_terminal::init()?;

    let result = App::new().run(&mut terminal);

    tui_terminal::restore()?;

    result?;
    Ok(())
}
