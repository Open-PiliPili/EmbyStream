use embystream::{info_log, logger::*};

#[allow(unused_imports)]
use embystream::{
    AlistClient, EmbyClient, ClientBuilder, CurlPlugin, MarkdownV2Builder, TelegramClient, TextMessage,
};

fn setup_logger() {
    Logger::builder().with_level(LogLevel::Debug).build()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_logger();

    info_log!("Starting Emby Stream application");

    /*
    let telegram_client = ClientBuilder::<TelegramClient>::new()
        .with_plugin(CurlPlugin)
        .build();

    let md_text = MarkdownV2Builder::new().text("Hello, Telegram!").build();
    let text = TextMessage::new(md_text);
    let response = telegram_client
        .send_text(
            "6XXXXXX:AAEXXXXXXXXXXXXXXXXXXXXXXXX8Cxc",
            "-1XXXXXXXXX1",
            text,
        )
        .await?;
    info_log!("Telegram Response: {:?}", response);

    let alist_client = ClientBuilder::<AlistClient>::new()
        .with_plugin(CurlPlugin)
        .build();

    let alist_token = "alist-XXXXXXXXX";
    let file_link = alist_client
        .fetch_file_link(
            "http://XXXXXXXXXXXXX:5244/",
            alist_token,
            "/115/影视/国产/一把青 (2015)/Season 1/一把青 (2015) - S01E01 - 1080p WEB-DL.mkv",
        )
        .await?;
    info_log!("File_link: {}", file_link);

    let base_url = "https://bps8m.onyra.cc/";
    let emby_api_key = "***************************";
    let emby_client = ClientBuilder::<EmbyClient>::new()
        .with_plugin(CurlPlugin)
        .build();
    let playback_result = emby_client.playback_info(
        base_url,
        emby_api_key,
        "197542",
        "9b587cca92a81557732604ce21af6094",
    ).await?;
    info_log!("playbackInfo: {:?}", playback_result.media_sources[0].path);
    let user_result = emby_client.get_user(
        base_url,
        emby_api_key,
        "723b389488a54e03b69404bdbcd628d3"
    ).await?;
    info_log!("user: {:?}", user_result.name);
     */

    Ok(())
}
