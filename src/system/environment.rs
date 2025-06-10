#[derive(Debug)]
pub enum Environment {
    Docker,
    Linux,
    MacOS,
    Windows,
    Unknown,
}
