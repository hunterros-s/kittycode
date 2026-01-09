use anyhow::Result;
use tui_app::protocol::OpenAIClient;
use tui_app::App;

#[tokio::main]
async fn main() -> Result<()> {
    let client = OpenAIClient::from_env()?;

    let mut terminal = tui_terminal::init()?;
    let result = App::new(client).run(&mut terminal).await;
    tui_terminal::restore()?;

    result
}
