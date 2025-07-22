pub mod download;
pub mod emby;
pub mod openlist;
pub mod telegram;

pub use emby::{
    API as EmbyAPI, Operation as EmbyOperation,
    response::{PlaybackInfo, User},
};

pub use telegram::{
    API as TelegramAPI, Operation as TelegramOperation,
    request::{PhotoMessage, TextMessage},
    response::{MessageResult, Response},
};

pub use openlist::{
    API as OpenListAPI, Operation as OpenListOperation,
    response::{FileData, FileResponse, LinkData, LinkResponse},
};

pub use download::API as DownloadAPI;
