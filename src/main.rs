use embystream::{info_log, logger::*};

fn setup_logger() {
    Logger::builder().with_level(LogLevel::Debug).build()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_logger();

    info_log!("Starting Emby Stream application");

    /* Test Telegream client
    let telegram_client = ClientBuilder::<TelegramClient>::new()
        .with_plugin(CurlPlugin)
        .build();

    let md_text = MarkdownV2Builder::new().text("Hello, Telegram!").build();
    let text = TextMessage::new(md_text);
    let response = telegram_client
        .send_text(
            "6XXXXXX:AAEXXXXXXXXXXXXXXXXXXXXXXXX8Cxc",
            "-1XXXXXXXXX1",
            text
        )
        .await?;
    info_log!("Telegram Response: {:?}", response);
     */

    /*
    let alist_client = ClientBuilder::<AlistClient>::new()
        .with_plugin(CurlPlugin)
        .build();

    let alist_token = "alist-XXXXXXXXX";
    let file_link = alist_client.fetch_file_link(
        "http://XXXXXXXXXXXXX:5244/",
        alist_token,
        "/115/影视/国产/一把青 (2015)/Season 1/一把青 (2015) - S01E01 - 1080p WEB-DL.mkv"
    ).await?;
    info_log!("File_link: {}", file_link);
    */

    Ok(())
}