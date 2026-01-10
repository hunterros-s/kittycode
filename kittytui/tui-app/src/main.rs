use anyhow::Result;
use tui_app::protocol::OpenAIClient;
use tui_app::App;
use tui_terminal::Tui;

#[tokio::main]
async fn main() -> Result<()> {
    let client = OpenAIClient::from_env()?;

    // Inline mode: 5 lines for input, rest for scrollback history
    let tui = Tui::new_inline(5)?;
    let result = App::new(client).run(tui).await;

    result
}
