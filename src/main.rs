#[allow(warnings)]
use std::{
    collections::HashMap,
    io::SeekFrom,
    path::PathBuf,
    sync::Arc
};

#[allow(warnings)]
use tokio::{
    io::AsyncSeekExt,
    time::{
        Duration,
        sleep,
        timeout
    }
};

#[allow(warnings)]
use embystream::{
    debug_log,
    info_log,
    logger::*,
    AlistClient,
    CryptoCacheManager,
    EmbyClient,
    ClientBuilder,
    CurlPlugin,
    MarkdownV2Builder,
    TelegramClient,
    TextMessage,
    FileCache
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

    let base_url = "https://**********/";
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

    let cache_manager = CryptoCacheManager::new(
        5000,
        60 * 60,
        5000,
        60 * 60
    );

    cache_manager
        .encrypted_cache()
        .insert("item123_source456".to_string(), "base64_encoded_json_string".to_string());
    cache_manager
        .encrypted_cache()
        .insert("item123_source456".to_string(), "base641_encoded_json_string".to_string());
    cache_manager
        .encrypted_cache()
        .insert("item456_source456".to_string(), "base642_encoded_json_string".to_string());

    let base64_key = "base64_encoded_json_string".to_string();
    let mut decrypted_value: HashMap<String, String> = HashMap::new();
    decrypted_value.insert("key1".to_string(), "value1".to_string());
    decrypted_value.insert("key2".to_string(), "value2".to_string());
    decrypted_value.insert("key3".to_string(), "value3".to_string());
    cache_manager
        .decrypted_cache()
        .insert(base64_key.clone(), decrypted_value);
    cache_manager
        .decrypted_cache()
        .get::<HashMap<String, String>>(&base64_key);

    let cache = FileCache::builder()
        .with_max_capacity(2000)
        .build()
        .await;
    let file_path = PathBuf::from("/Users/xxxxxx/Downloads/test.mov");

    // Mock user1 get filehandle and seek
    let handle1 = cache.get_file(file_path.clone()).await.unwrap();
    debug_log!("Attempting to seek to position 1000...");
    let seek_result = timeout(Duration::from_secs(5), async {
        let mut file = handle1.write().await;
        file.seek(SeekFrom::Start(1000)).await
    }).await;
    match seek_result {
        Ok(Ok(position)) => {
            debug_log!("User1: Successfully seeked to position: {}", position);
        }
        Ok(Err(e)) => {
            debug_log!("User1: Failed to seek to position 1000: {}", e);
        }
        Err(_) => {
            debug_log!("User1: Seek operation timed out after 5 seconds");
        }
    }
    cache.release_file(file_path.clone()).await;
    sleep(Duration::from_secs(1)).await;

    // Mock user2 get filehandle and seek
    let handle2 = cache.get_file(file_path.clone()).await.unwrap();
    debug_log!("Attempting to seek to position 2000...");
    let seek_result2 = timeout(Duration::from_secs(5), async {
        let mut file = handle2.write().await;
        file.seek(SeekFrom::Start(2000)).await
    }).await;
    match seek_result2 {
        Ok(Ok(position)) => {
            debug_log!("User2: Successfully seeked to position: {}", position);
        }
        Ok(Err(e)) => {
            debug_log!("User2: Failed to seek to position 2000: {}", e);
        }
        Err(_) => {
            debug_log!("User2: Seek operation timed out after 5 seconds");
        }
    }

    // Mock user3 get filehandle and seek
    let handle3 = cache.get_file(file_path.clone()).await.unwrap();
    debug_log!("Attempting to seek to position 3000...");
    let seek_result3 = timeout(Duration::from_secs(5), async {
        let mut file = handle3.write().await;
        file.seek(SeekFrom::Start(3000)).await
    }).await;
    match seek_result3 {
        Ok(Ok(position)) => {
            debug_log!("User2: Successfully seeked to position: {}", position);
        }
        Ok(Err(e)) => {
            debug_log!("User2: Failed to seek to position 3000: {}", e);
        }
        Err(_) => {
            debug_log!("User2: Seek operation timed out after 5 seconds");
        }
    }

    for index in 0..5 {
        let metadata = cache.get_metadata(&file_path).await;
        debug_log!("metadata-{}: {:?}", index, metadata);
        sleep(Duration::from_millis(100)).await;
    }
    */

    Ok(())
}
