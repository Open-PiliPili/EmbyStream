fn main() {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        println!("cargo:rustc-env=FFMPEG_DIR=/opt/homebrew");
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        println!("cargo:rustc-env=FFMPEG_DIR=/usr/local");
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        println!("cargo:rustc-env=FFMPEG_DIR=/opt/ffmpeg_arm");
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        println!("cargo:rustc-env=FFMPEG_DIR=/opt/ffmpeg_x86");
    }
}