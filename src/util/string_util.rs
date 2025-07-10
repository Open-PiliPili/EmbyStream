use md5;

pub struct StringUtil;

impl StringUtil {

    pub fn trim_trailing_slashes(input: &str) -> &str {
        input.trim_end_matches('/')
    }

    pub fn md5(input: &str) -> String {
        if input.is_empty() {
            return "".to_string();
        }
        let digest = md5::compute(input.as_bytes());
        format!("{:x}", digest)
    }
}