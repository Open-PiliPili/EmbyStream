#[derive(Debug)]
pub enum Environment {
    Unknown,
    Docker,
    Linux,
    MacOS,
    Windows
}