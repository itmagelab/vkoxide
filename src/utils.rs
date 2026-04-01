use serde_json::Value;

pub fn get_i64(v: &[Value], i: usize) -> Option<i64> {
    v.get(i)?.as_i64()
}

pub fn get_str(v: &[Value], i: usize) -> Option<String> {
    v.get(i)?.as_str().map(|s| s.to_string())
}
