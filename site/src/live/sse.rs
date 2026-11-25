//! A server-sent events parser for the Messages API stream.
//!
//! Bytes arrive in arbitrary chunks; [`Parser::feed`] buffers them and returns each
//! complete event. It follows the parts of the SSE format the API uses: `event:` and
//! `data:` fields, comments, and `\n`, `\r\n` or `\r` line endings.

/// One event: its name (`message` when the stream gives none) and its data.
#[derive(Debug, PartialEq, Eq)]
pub struct Event {
    pub name: String,
    pub data: String,
}

/// The longest line accepted, so a broken stream cannot grow the buffer forever.
const MAX_LINE: usize = 4 * 1024 * 1024;

/// An incremental parser.
#[derive(Default)]
pub struct Parser {
    /// Bytes after the last complete line.
    buffer: Vec<u8>,
    /// The fields of the event being read.
    name: Option<String>,
    data: Option<String>,
    /// The last byte fed was a `\r`, so a leading `\n` in the next chunk ends nothing.
    after_cr: bool,
}

/// The stream broke the format.
#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    LineTooLong,
    NotUtf8,
}

impl Parser {
    /// Adds `chunk` and returns the events it completed.
    pub fn feed(&mut self, chunk: &[u8]) -> Result<Vec<Event>, ParseError> {
        let mut events = Vec::new();
        for &byte in chunk {
            if self.after_cr {
                self.after_cr = false;
                if byte == b'\n' {
                    continue;
                }
            }
            match byte {
                b'\n' | b'\r' => {
                    self.after_cr = byte == b'\r';
                    let line = std::mem::take(&mut self.buffer);
                    let line = String::from_utf8(line).map_err(|_| ParseError::NotUtf8)?;
                    if let Some(event) = self.line(&line) {
                        events.push(event);
                    }
                }
                _ => {
                    if self.buffer.len() == MAX_LINE {
                        return Err(ParseError::LineTooLong);
                    }
                    self.buffer.push(byte);
                }
            }
        }
        Ok(events)
    }

    /// Handles one line, returning the event a blank line completes.
    fn line(&mut self, line: &str) -> Option<Event> {
        if line.is_empty() {
            let data = self.data.take();
            let name = self.name.take();
            return data.map(|data| Event {
                name: name.unwrap_or_else(|| "message".to_owned()),
                data,
            });
        }
        if line.starts_with(':') {
            return None;
        }
        let (field, value) = line.split_once(':').unwrap_or((line, ""));
        let value = value.strip_prefix(' ').unwrap_or(value);
        match field {
            "event" => self.name = Some(value.to_owned()),
            "data" => match &mut self.data {
                Some(data) => {
                    data.push('\n');
                    data.push_str(value);
                }
                None => self.data = Some(value.to_owned()),
            },
            // `id` and `retry` are for reconnecting, which this client does not do.
            _ => {}
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(name: &str, data: &str) -> Event {
        Event {
            name: name.to_owned(),
            data: data.to_owned(),
        }
    }

    #[test]
    fn parses_events_split_across_chunks() {
        let stream = "event: message_start\ndata: {\"type\":\"message_start\"}\n\n\
                      event: ping\ndata: {\"type\": \"ping\"}\n\n";
        // Every split point, including inside a line and between `\n`s.
        for split in 0..stream.len() {
            let mut parser = Parser::default();
            let mut events = parser.feed(&stream.as_bytes()[..split]).unwrap();
            events.extend(parser.feed(&stream.as_bytes()[split..]).unwrap());
            assert_eq!(
                events,
                [
                    event("message_start", "{\"type\":\"message_start\"}"),
                    event("ping", "{\"type\": \"ping\"}"),
                ],
                "split at {split}"
            );
        }
    }

    #[test]
    fn handles_crlf_comments_and_multiline_data() {
        let mut parser = Parser::default();
        let events = parser
            .feed(b": keep-alive\r\nevent: x\r\ndata: a\r\ndata: b\r\n\r\ndata:c\r\n\r\n")
            .unwrap();
        assert_eq!(events, [event("x", "a\nb"), event("message", "c")]);
    }

    #[test]
    fn a_crlf_split_between_chunks_ends_one_line() {
        let mut parser = Parser::default();
        let mut events = parser.feed(b"data: a\r").unwrap();
        events.extend(parser.feed(b"\n\r\n").unwrap());
        assert_eq!(events, [event("message", "a")]);
    }

    #[test]
    fn utf8_split_across_chunks_survives() {
        let text = "data: caf\u{e9} \u{1f980}\n\n".as_bytes();
        let mut parser = Parser::default();
        let mut events = Vec::new();
        for byte in text {
            events.extend(parser.feed(&[*byte]).unwrap());
        }
        assert_eq!(events, [event("message", "caf\u{e9} \u{1f980}")]);
    }
}
