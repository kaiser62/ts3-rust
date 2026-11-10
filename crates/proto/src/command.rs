use std::collections::HashMap;
use std::fmt;

use base64::Engine as _;
use sha1::{Digest, Sha1};
use thiserror::Error;

/// Errors that can occur when parsing or handling TeamSpeak 3 commands.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CommandError {
    #[error("command string is empty")]
    EmptyCommand,
    #[error("invalid command format: {0}")]
    InvalidFormat(String),
}

/// Escapes a string according to the TeamSpeak 3 command parameter escaping table.
///
/// Escaped characters:
/// - `\` (0x5C) -> `\\`
/// - `/` (0x2F) -> `\/`
/// - ` ` (0x20) -> `\s`
/// - `|` (0x7C) -> `\p`
/// - `\x0C` (Form Feed) -> `\f`
/// - `\n` (0x0A) -> `\n`
/// - `\r` (0x0D) -> `\r`
/// - `\t` (0x09) -> `\t`
/// - `\x0B` (Vertical Tab) -> `\v`
/// - `\x07` (Bell / Alert) -> `\a`
/// - `\x08` (Backspace) -> `\b`
pub fn escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '/' => out.push_str("\\/"),
            ' ' => out.push_str("\\s"),
            '|' => out.push_str("\\p"),
            '\x0C' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x0B' => out.push_str("\\v"),
            '\x07' => out.push_str("\\a"),
            '\x08' => out.push_str("\\b"),
            other => out.push(other),
        }
    }
    out
}

/// Unescapes a TeamSpeak 3 command parameter value according to the TS3 escape table.
///
/// Trailing single `\` or unknown `\x` sequences are handled gracefully:
/// - A trailing `\` at the end of the input is preserved as `\`.
/// - An unknown escape sequence `\x` preserves `x`.
pub fn unescape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('\\') => out.push('\\'),
                Some('/') => out.push('/'),
                Some('s') => out.push(' '),
                Some('p') => out.push('|'),
                Some('f') => out.push('\x0C'),
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('v') => out.push('\x0B'),
                Some('a') => out.push('\x07'),
                Some('b') => out.push('\x08'),
                Some(unknown) => out.push(unknown),
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// Hashes a cleartext password for TeamSpeak 3 full-client commands (`SHA-1` + `Base64`).
///
/// Returns `""` if `cleartext.is_empty()`.
pub fn hash_password(cleartext: &str) -> String {
    if cleartext.is_empty() {
        return String::new();
    }
    let digest = Sha1::digest(cleartext.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(digest)
}

/// Builder for outgoing TeamSpeak 3 commands.
///
/// Supports single parameters, stateful or explicit pipe-separated multi-item parameter sets,
/// and trailing `-switch` options.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Command {
    name: String,
    single_params: Vec<(String, String)>,
    items: Vec<Vec<(String, String)>>,
    building_items: bool,
    switches: Vec<String>,
}

/// Type alias so both `Command` and `CommandBuilder` can be used interchangeably.
pub type CommandBuilder = Command;

impl Command {
    /// Creates a new command with the given command name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            single_params: Vec::new(),
            items: Vec::new(),
            building_items: false,
            switches: Vec::new(),
        }
    }

    /// Returns the command name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Appends a key-value parameter (escaping the value automatically).
    ///
    /// If [`next_item`](Self::next_item) has been called, this appends to the current item
    /// in the multi-item parameter list; otherwise it appends to the single parameters list.
    pub fn arg(mut self, key: impl Into<String>, value: impl ToString) -> Self {
        let k = key.into();
        let v = escape(&value.to_string());
        if self.building_items {
            if let Some(last) = self.items.last_mut() {
                last.push((k, v));
            } else {
                self.items.push(vec![(k, v)]);
            }
        } else {
            self.single_params.push((k, v));
        }
        self
    }

    /// Appends a boolean parameter (`true` -> `"1"`, `false` -> `"0"`).
    pub fn arg_bool(self, key: impl Into<String>, value: bool) -> Self {
        self.arg(key, if value { "1" } else { "0" })
    }

    /// Appends a pre-escaped or raw key-value parameter without escaping the value again.
    pub fn arg_raw(mut self, key: impl Into<String>, raw_value: impl Into<String>) -> Self {
        let k = key.into();
        let v = raw_value.into();
        if self.building_items {
            if let Some(last) = self.items.last_mut() {
                last.push((k, v));
            } else {
                self.items.push(vec![(k, v)]);
            }
        } else {
            self.single_params.push((k, v));
        }
        self
    }

    /// Starts a new pipe-separated item record.
    ///
    /// If single parameters were already added before the first `next_item()` call and no
    /// explicit items exist yet, those parameters become the first item (`item 0`), and
    /// subsequent `.arg(...)` calls populate the new item.
    pub fn next_item(mut self) -> Self {
        if !self.building_items {
            self.building_items = true;
            if !self.single_params.is_empty() && self.items.is_empty() {
                let first = std::mem::take(&mut self.single_params);
                self.items.push(first);
            }
        }
        self.items.push(Vec::new());
        self
    }

    /// Appends a complete multi-parameter item record (`k1=v1 k2=v2`) to the pipe-separated list.
    /// Values are escaped automatically.
    pub fn item<I, K, V>(mut self, params: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: ToString,
    {
        let record: Vec<(String, String)> = params
            .into_iter()
            .map(|(k, v)| (k.into(), escape(&v.to_string())))
            .collect();
        if !record.is_empty() {
            self.items.push(record);
        }
        self
    }

    /// Alias for [`item`](Self::item).
    pub fn push_item<I, K, V>(self, params: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: ToString,
    {
        self.item(params)
    }

    /// Appends a column of values for a single key across multiple pipe-separated items
    /// (similar to `CommandMultiParameter` in `TSLib`).
    pub fn multi_arg<I, V>(mut self, key: impl Into<String>, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: ToString,
    {
        let key_str = key.into();
        for (idx, val) in values.into_iter().enumerate() {
            let pair = (key_str.clone(), escape(&val.to_string()));
            if idx < self.items.len() {
                self.items[idx].push(pair);
            } else {
                self.items.push(vec![pair]);
            }
        }
        self
    }

    /// Appends a command switch/option (e.g. `.switch("away")` emits `-away` at the end).
    /// If the passed string already starts with `'-'`, the leading `'-'` is stripped before storing.
    pub fn switch(mut self, opt: impl Into<String>) -> Self {
        let s = opt.into();
        let trimmed = s.strip_prefix('-').unwrap_or(&s).to_string();
        if !trimmed.is_empty() {
            self.switches.push(trimmed);
        }
        self
    }

    /// Serializes the command to its wire-format string representation:
    /// 1. `<CommandName>` (if non-empty)
    /// 2. Single parameters (`<key>=<escaped_val>`), space-separated
    /// 3. Multi-item parameter sets (`<k1>=<v1> <k2>=<v2>|<k1>=<v1b> <k2>=<v2b>`), joined by `|`
    /// 4. Switches/options (`-<option>`), space-separated at the end
    pub fn build(&self) -> String {
        let mut out = String::new();

        if !self.name.is_empty() {
            out.push_str(&escape(&self.name));
        }

        for (k, v) in &self.single_params {
            if !out.is_empty() {
                out.push(' ');
            }
            if v.is_empty() {
                out.push_str(k);
                out.push('=');
            } else {
                out.push_str(k);
                out.push('=');
                out.push_str(v);
            }
        }

        let non_empty_items: Vec<&Vec<(String, String)>> =
            self.items.iter().filter(|it| !it.is_empty()).collect();

        if !non_empty_items.is_empty() {
            if !out.is_empty() {
                out.push(' ');
            }
            for (item_idx, item) in non_empty_items.iter().enumerate() {
                if item_idx > 0 {
                    out.push('|');
                }
                for (param_idx, (k, v)) in item.iter().enumerate() {
                    if param_idx > 0 {
                        out.push(' ');
                    }
                    out.push_str(k);
                    out.push('=');
                    out.push_str(v);
                }
            }
        }

        for sw in &self.switches {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push('-');
            out.push_str(sw);
        }

        out
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.build())
    }
}

impl From<Command> for String {
    fn from(cmd: Command) -> Self {
        cmd.build()
    }
}

/// Parsed representation of an incoming S2C or C2S TeamSpeak 3 command/notification string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCommand {
    /// Command or notification name. Empty (`""`) if the line is an unnamed direct data response.
    pub name: String,
    /// Pipe-separated items, each containing unescaped `key -> value` pairs as received on the wire.
    pub items: Vec<HashMap<String, String>>,
    /// Any `-switch` flags encountered on the command.
    pub switches: Vec<String>,
}

impl ParsedCommand {
    /// Parses a raw S2C or C2S command/notification string into a [`ParsedCommand`].
    pub fn parse(raw: &str) -> Result<Self, CommandError> {
        let trimmed = raw.trim_matches(|c| c == ' ' || c == '\r' || c == '\n');
        if trimmed.is_empty() {
            return Err(CommandError::EmptyCommand);
        }

        // Inspect the first space-delimited token to determine if the command is named or unnamed.
        let (first_token, rest_after_first) = match trimmed.find(' ') {
            Some(idx) => (&trimmed[..idx], Some(&trimmed[idx + 1..])),
            None => (trimmed, None),
        };

        // If the first token contains '=' or '|', it's an unnamed direct response list.
        // If it starts with '-', it's a switch without a command name.
        let (name, body) =
            if first_token.contains('=') || first_token.contains('|') || first_token.starts_with('-')
            {
                (String::new(), Some(trimmed))
            } else {
                (unescape(first_token), rest_after_first)
            };

        let mut items = Vec::new();
        let mut switches = Vec::new();

        if let Some(body_str) = body {
            let body_trimmed = body_str.trim_matches(' ');
            if !body_trimmed.is_empty() {
                for pipe_slice in body_trimmed.split('|') {
                    let mut map = HashMap::new();
                    for token in pipe_slice.split(' ') {
                        if token.is_empty() {
                            continue;
                        }
                        if let Some(sw) = token.strip_prefix('-') {
                            if !sw.contains('=') && !sw.is_empty() {
                                switches.push(sw.to_string());
                                continue;
                            }
                        }
                        match token.find('=') {
                            Some(eq_idx) => {
                                let key = &token[..eq_idx];
                                let raw_val = &token[eq_idx + 1..];
                                if !key.is_empty() {
                                    map.insert(key.to_string(), unescape(raw_val));
                                }
                            }
                            None => {
                                map.insert(token.to_string(), String::new());
                            }
                        }
                    }
                    items.push(map);
                }
            }
        }

        if items.is_empty() {
            items.push(HashMap::new());
        }

        Ok(Self {
            name,
            items,
            switches,
        })
    }

    /// Returns the items list with header fields from `items[0]` inherited into `items[1..]`
    /// (matching TeamSpeak 3's pipe-list `Expand` compression where identical fields across a batch
    /// are sent only in the first record).
    pub fn expanded_items(&self) -> Vec<HashMap<String, String>> {
        if self.items.len() <= 1 {
            return self.items.clone();
        }

        let mut expanded = self.items.clone();
        let header = &self.items[0];
        for item in expanded.iter_mut().skip(1) {
            for (k, v) in header {
                if !item.contains_key(k) {
                    item.insert(k.clone(), v.clone());
                }
            }
        }
        expanded
    }

    /// Gets a string value from the first item (`items[0]`).
    pub fn get(&self, key: &str) -> Option<&str> {
        self.items.first()?.get(key).map(|s| s.as_str())
    }

    /// Gets and parses a `u64` value from the first item (`items[0]`).
    pub fn get_u64(&self, key: &str) -> Option<u64> {
        self.get(key)?.parse::<u64>().ok()
    }

    /// Gets and parses an `i32` value from the first item (`items[0]`).
    ///
    /// Also handles unsigned 32-bit decimal values greater than `i32::MAX` (such as TS3 icon CRC32
    /// hashes formatted as positive `u32`/`u64` decimals) by wrapping them to `i32`.
    pub fn get_i32(&self, key: &str) -> Option<i32> {
        let s = self.get(key)?;
        s.parse::<i32>()
            .or_else(|_| s.parse::<u64>().map(|v| v as i32))
            .ok()
    }

    /// Gets and parses a `u32` value from the first item (`items[0]`).
    pub fn get_u32(&self, key: &str) -> Option<u32> {
        self.get(key)?.parse::<u32>().ok()
    }

    /// Gets and parses a `u16` value from the first item (`items[0]`).
    pub fn get_u16(&self, key: &str) -> Option<u16> {
        self.get(key)?.parse::<u16>().ok()
    }

    /// Gets and parses a boolean value from the first item (`items[0]`).
    ///
    /// Follows TS3 semantics: `"1"` / `"true"` (or any non-empty string not starting with `'0'` or `"false"`)
    /// is `true`, and `"0"` / `"false"` is `false`. Empty strings return `None`.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        let s = self.get(key)?;
        if s.is_empty() {
            return None;
        }
        if s.eq_ignore_ascii_case("false") || s.as_bytes()[0] == b'0' {
            Some(false)
        } else if s.eq_ignore_ascii_case("true") || s.as_bytes()[0] != b'0' {
            Some(true)
        } else {
            None
        }
    }
}

/// Structured representation of a TeamSpeak 3 `error` response packet.
///
/// Example wire lines:
/// - `error id=0 msg=ok return_code=1`
/// - `error id=2568 msg=insufficient\sclient\spermissions failed_permid=139 return_code=4`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TsErrorResponse {
    pub id: u32,
    pub msg: String,
    pub return_code: Option<String>,
    pub extra_msg: Option<String>,
    pub failed_permid: Option<u32>,
}

impl TsErrorResponse {
    /// Extracts a [`TsErrorResponse`] if `cmd.name == "error"`.
    pub fn from_command(cmd: &ParsedCommand) -> Option<Self> {
        if cmd.name != "error" {
            return None;
        }
        let id = cmd.get_u32("id").unwrap_or(0);
        let msg = cmd.get("msg").unwrap_or("").to_string();
        let return_code = cmd.get("return_code").map(|s| s.to_string());
        let extra_msg = cmd.get("extra_msg").map(|s| s.to_string());
        let failed_permid = cmd.get_u32("failed_permid");

        Some(Self {
            id,
            msg,
            return_code,
            extra_msg,
            failed_permid,
        })
    }

    /// Returns `true` if the error code is `0` (`ok`).
    pub fn is_ok(&self) -> bool {
        self.id == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_and_unescape_all_11_special_chars() {
        let cases = [
            ('\\', "\\\\"),
            ('/', "\\/"),
            (' ', "\\s"),
            ('|', "\\p"),
            ('\x0C', "\\f"),
            ('\n', "\\n"),
            ('\r', "\\r"),
            ('\t', "\\t"),
            ('\x0B', "\\v"),
            ('\x07', "\\a"),
            ('\x08', "\\b"),
        ];

        for (raw_ch, expected_esc) in cases {
            let s = raw_ch.to_string();
            assert_eq!(
                escape(&s),
                expected_esc,
                "failed escaping {:?}",
                raw_ch as u32
            );
            assert_eq!(
                unescape(expected_esc),
                s,
                "failed unescaping {}",
                expected_esc
            );
        }

        // Combined string containing all 11 special characters + normal text
        let combined_raw = "Hello\\World/Team Speak|Pipe\x0CForm\nLine\rCarriage\tTab\x0BVert\x07Bell\x08Back!";
        let expected_wire = "Hello\\\\World\\/Team\\sSpeak\\pPipe\\fForm\\nLine\\rCarriage\\tTab\\vVert\\aBell\\bBack!";
        assert_eq!(escape(combined_raw), expected_wire);
        assert_eq!(unescape(expected_wire), combined_raw);
    }

    #[test]
    fn test_unescape_graceful_edge_cases() {
        // Trailing backslash preserved
        assert_eq!(unescape("abc\\"), "abc\\");
        assert_eq!(unescape("\\"), "\\");
        // Unknown escape \x preserves 'x'
        assert_eq!(unescape("hello\\xworld\\z"), "helloxworldz");
    }

    #[test]
    fn test_hash_password() {
        assert_eq!(hash_password(""), "");
        // SHA1("secret") = e5e9fa1ba31ecd1ae84f75caaa474f3a663f05f4 -> Base64 = "5en6G6MezRroT3XKqkdPOmY/BfQ="
        assert_eq!(hash_password("secret"), "5en6G6MezRroT3XKqkdPOmY/BfQ=");
    }

    #[test]
    fn test_command_builder_single_and_switches() {
        let cmd = CommandBuilder::new("clientinit")
            .arg("client_nickname", "Rust Bot")
            .arg("client_version", "3.?.? [Build: 1568021092]")
            .arg_bool("client_input_hardware", true)
            .arg_bool("client_output_hardware", false)
            .switch("away")
            .switch("-voice")
            .build();

        assert_eq!(
            cmd,
            "clientinit client_nickname=Rust\\sBot client_version=3.?.?\\s[Build:\\s1568021092] client_input_hardware=1 client_output_hardware=0 -away -voice"
        );
    }

    #[test]
    fn test_command_builder_multi_items() {
        // Using .item(...)
        let cmd1 = Command::new("channelsubscribe")
            .item([("cid", 1)])
            .item([("cid", 2)])
            .item([("cid", 3)])
            .arg("return_code", "1")
            .build();
        assert_eq!(cmd1, "channelsubscribe return_code=1 cid=1|cid=2|cid=3");

        // Using stateful .next_item()
        let cmd2 = Command::new("servergroupaddperm")
            .arg("sgid", 6)
            .item([
                ("permid", 10),
                ("permvalue", 75),
                ("permnegated", 0),
                ("permskip", 0),
            ])
            .next_item()
            .arg("permid", 11)
            .arg("permvalue", 50)
            .arg("permnegated", 0)
            .arg("permskip", 0)
            .build();
        assert_eq!(
            cmd2,
            "servergroupaddperm sgid=6 permid=10 permvalue=75 permnegated=0 permskip=0|permid=11 permvalue=50 permnegated=0 permskip=0"
        );

        // Using stateful .arg() followed by .next_item()
        let cmd3 = Command::new("clientkick")
            .arg("clid", 5)
            .next_item()
            .arg("clid", 6)
            .build();
        assert_eq!(cmd3, "clientkick clid=5|clid=6");

        // Using .multi_arg()
        let cmd4 = Command::new("channelunsubscribe")
            .multi_arg("cid", [10, 20, 30])
            .build();
        assert_eq!(cmd4, "channelunsubscribe cid=10|cid=20|cid=30");
    }

    #[test]
    fn test_parsed_command_initserver() {
        let raw = "initserver virtualserver_name=TeamSpeak\\s]\\p[\\sServer virtualserver_maxclients=32 aclid=5 pv=6 virtualserver_icon_id=3054198960 virtualserver_flag_password=1";
        let parsed = ParsedCommand::parse(raw).expect("should parse initserver");

        assert_eq!(parsed.name, "initserver");
        assert_eq!(parsed.items.len(), 1);
        assert_eq!(
            parsed.get("virtualserver_name"),
            Some("TeamSpeak ]|[ Server")
        );
        assert_eq!(parsed.get_u32("virtualserver_maxclients"), Some(32));
        assert_eq!(parsed.get_u16("aclid"), Some(5));
        assert_eq!(parsed.get_u64("pv"), Some(6));
        assert_eq!(parsed.get_bool("virtualserver_flag_password"), Some(1 == 1));
        // 3054198960u32 == 0xB60B60B0 -> as i32 is -1240768336
        assert_eq!(
            parsed.get_i32("virtualserver_icon_id"),
            Some(3054198960u32 as i32)
        );
    }

    #[test]
    fn test_parsed_command_pipe_list_expansion() {
        let raw = "notifycliententerview cfid=0 ctid=1 reasonid=0 clid=5 client_nickname=Alice\\sSmith client_input_muted=0|clid=6 client_nickname=Bob\\sJones client_input_muted=1";
        let parsed = ParsedCommand::parse(raw).expect("should parse notifycliententerview");

        assert_eq!(parsed.name, "notifycliententerview");
        assert_eq!(parsed.items.len(), 2);

        // Raw items[1] does not have cfid, ctid, reasonid before expansion
        assert_eq!(parsed.items[1].get("cfid"), None);

        // Expanded items inherit cfid, ctid, reasonid from items[0]
        let expanded = parsed.expanded_items();
        assert_eq!(expanded.len(), 2);
        assert_eq!(expanded[0].get("cfid").map(String::as_str), Some("0"));
        assert_eq!(expanded[0].get("ctid").map(String::as_str), Some("1"));
        assert_eq!(expanded[0].get("clid").map(String::as_str), Some("5"));
        assert_eq!(
            expanded[0].get("client_nickname").map(String::as_str),
            Some("Alice Smith")
        );

        assert_eq!(expanded[1].get("cfid").map(String::as_str), Some("0"));
        assert_eq!(expanded[1].get("ctid").map(String::as_str), Some("1"));
        assert_eq!(expanded[1].get("reasonid").map(String::as_str), Some("0"));
        assert_eq!(expanded[1].get("clid").map(String::as_str), Some("6"));
        assert_eq!(
            expanded[1].get("client_nickname").map(String::as_str),
            Some("Bob Jones")
        );
        assert_eq!(
            expanded[1].get("client_input_muted").map(String::as_str),
            Some("1")
        );
    }

    #[test]
    fn test_parsed_command_unnamed_direct_response() {
        let raw = "cid=1 clid=5 client_database_id=2 client_nickname=Tester|cid=1 clid=6 client_database_id=3 client_nickname=Other";
        let parsed = ParsedCommand::parse(raw).expect("should parse unnamed response");

        assert_eq!(parsed.name, "");
        assert_eq!(parsed.items.len(), 2);
        assert_eq!(parsed.get_u64("cid"), Some(1));
        assert_eq!(parsed.get_u16("clid"), Some(5));
        assert_eq!(parsed.get("client_nickname"), Some("Tester"));
        assert_eq!(
            parsed.items[1].get("client_nickname").map(String::as_str),
            Some("Other")
        );
    }

    #[test]
    fn test_ts_error_response() {
        let ok_cmd = ParsedCommand::parse("error id=0 msg=ok return_code=1").unwrap();
        let ok_err = TsErrorResponse::from_command(&ok_cmd).expect("should extract error");
        assert!(ok_err.is_ok());
        assert_eq!(ok_err.id, 0);
        assert_eq!(ok_err.msg, "ok");
        assert_eq!(ok_err.return_code.as_deref(), Some("1"));
        assert_eq!(ok_err.failed_permid, None);
        assert_eq!(ok_err.extra_msg, None);

        let fail_cmd = ParsedCommand::parse(
            "error id=2568 msg=insufficient\\sclient\\spermissions failed_permid=139 return_code=4 extra_msg=need\\smore\\spower",
        )
        .unwrap();
        let fail_err = TsErrorResponse::from_command(&fail_cmd).expect("should extract error");
        assert!(!fail_err.is_ok());
        assert_eq!(fail_err.id, 2568);
        assert_eq!(fail_err.msg, "insufficient client permissions");
        assert_eq!(fail_err.failed_permid, Some(139));
        assert_eq!(fail_err.return_code.as_deref(), Some("4"));
        assert_eq!(fail_err.extra_msg.as_deref(), Some("need more power"));

        let non_err_cmd = ParsedCommand::parse("channellistfinished").unwrap();
        assert!(TsErrorResponse::from_command(&non_err_cmd).is_none());
    }
}
