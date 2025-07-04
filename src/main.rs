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
    EmbyClient,
    ClientBuilder,
    CurlPlugin,
    MarkdownV2Builder,
    TelegramClient,
    TextMessage,
    FileCache,
    GeneralCache
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


    crypto_cache_test().await;

    let cache = FileCache::new(100);
    let file_path = PathBuf::from("/Users/***/Downloads/test.mov");

    {
        file_cache_test(&cache, file_path.clone(), 2000).await;
        file_cache_test(&cache, file_path.clone(), 1000).await;
        let pool_count = cache.entry_pools.entry_count();
        debug_log!("all cache items: {} pools", pool_count);
    }

    sleep(Duration::from_secs(1)).await;
    {
        file_cache_test(&cache, file_path.clone(), 3000).await;
    }

    sleep(Duration::from_secs(20)).await;
    let pool_count = cache.entry_pools.entry_count();
    debug_log!("all cache items: {} pools", pool_count);
     */

    Ok(())
}

/*
#[allow(dead_code)]
#[derive(Debug, Clone)]
struct User {
    id: u32,
    username: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct ApiKey {
    key: String,
    permissions: Vec<String>,
}

async fn crypto_cache_test() {
    let cache = GeneralCache::new(3, 2);
    let user1 = User { id: 1, username: "alice".to_string() };
    let api_key1 = ApiKey { key: "secret123".to_string(), permissions: vec!["read".to_string()] };
    cache.insert("my_string".to_string(), "Hello, Moka!".to_string());
    cache.insert("user:1".to_string(), user1.clone());
    cache.insert("api_key:1".to_string(), api_key1.clone());

    debug_log!("Current cache size: {}", cache.len()); // 应为 3

    let retrieved_string: Option<String> = cache.get("my_string");
    debug_log!("Got String? {:?}", retrieved_string);

    let retrieved_user: Option<User> = cache.get("user:1");
    debug_log!("Got User? {:?}", retrieved_user);

    let wrong_type: Option<User> = cache.get("my_string");
    debug_log!("Got User from a String key? {:?}", wrong_type);

    let non_existent: Option<String> = cache.get("non_existent_key");
    debug_log!("Got non-existent key? {:?}", non_existent);

    let _ = cache.get::<String>("my_string");

    let user2 = User { id: 2, username: "bob".to_string() };
    debug_log!("Inserting a 4th item ('user:2')...");
    cache.insert("user:2".to_string(), user2);

    let evicted_user: Option<User> = cache.get("user:1");
    debug_log!("Is 'user:1' still in cache? {:?}", evicted_user);

    sleep(Duration::from_secs(3)).await;
    let expired_string: Option<String> = cache.get("my_string");
    debug_log!("Attempting to get 'my_string' after 3s: {:?}", expired_string);

    let expired_apikey: Option<ApiKey> = cache.get("api_key:1");
    debug_log!("Attempting to get 'api_key:1' after 3s: {:?}", expired_apikey);
}

async fn file_cache_test(cache: &FileCache, file_path: PathBuf, seek_start: u64) {
    let entry_result = cache.fetch_entry(&file_path).await;
    if entry_result.is_err() {
        debug_log!("Failed to fetch entry, cannot proceed.");
        return;
    }
    let entry = entry_result.unwrap();

    let metadata = cache.fetch_metadata(&file_path).await.unwrap();
    debug_log!("Successfully fetched metadata: {:?}", metadata);

    let pool_count = cache.get_pool_count();
    let metadata_count = cache.get_metadata_count();
    debug_log!("all cache items: {} pools, {} metadata entries", pool_count, metadata_count);

    debug_log!("Attempting to seek to position 1000...");

    let seek_result = timeout(Duration::from_secs(5), async {
        let handle_arc = entry.handle.clone();
        let mut file_guard = handle_arc.write().await;
        file_guard.seek(SeekFrom::Start(seek_start)).await
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

    debug_log!("Entry goes out of scope. No manual release needed.");
}
*/