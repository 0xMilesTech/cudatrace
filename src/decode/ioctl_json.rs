pub struct JsonObject {
    out: String,
    first: bool,
}

impl JsonObject {
    pub fn new() -> Self {
        Self {
            out: "{".to_owned(),
            first: true,
        }
    }

    pub fn field_str(&mut self, key: &str, value: &str) {
        self.push_field_key(key);
        self.out.push_str(&json_quote(value));
    }

    pub fn field_bool(&mut self, key: &str, value: bool) {
        self.push_field_key(key);
        self.out.push_str(if value { "true" } else { "false" });
    }

    pub fn field_i64(&mut self, key: &str, value: i64) {
        self.push_field_key(key);
        self.out.push_str(&value.to_string());
    }

    pub fn field_u64(&mut self, key: &str, value: u64) {
        self.push_field_key(key);
        self.out.push_str(&value.to_string());
    }

    pub fn field_raw(&mut self, key: &str, raw_json: &str) {
        self.push_field_key(key);
        self.out.push_str(raw_json);
    }

    pub fn finish(mut self) -> String {
        self.out.push('}');
        self.out
    }

    fn push_field_key(&mut self, key: &str) {
        if !self.first {
            self.out.push(',');
        }
        self.first = false;
        self.out.push_str(&json_quote(key));
        self.out.push(':');
    }
}

pub fn json_quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
