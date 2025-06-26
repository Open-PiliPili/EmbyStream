pub mod alist;
pub mod emby;
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

pub use alist::{
    API as AlistAPI, Operation as AlistOperation,
    response::{FileData, FileResponse, LinkData, LinkResponse},
};
