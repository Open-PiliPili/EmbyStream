pub struct StringUtil;

impl StringUtil {

    pub fn trim_trailing_slashes(input: &str) -> &str {
        input.trim_end_matches('/')
    }
}