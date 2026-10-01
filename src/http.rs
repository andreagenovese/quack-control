//! Just enough HTTP/1.1 for one page, one event stream and one JSON call.
//!
//! A request is read whole (line, headers, a `Content-Length` body), one
//! response is written, and the connection closes: no keep-alive, no
//! chunked bodies, no pipelining. Small limits everywhere, because the
//! other end is whatever is on the home network.

use std::io::{BufRead, Read, Write};

/// The request line and the headers together.
const MAX_HEAD: usize = 16 * 1024;
const MAX_HEADERS: usize = 64;
/// A call is a tool name and a few arguments.
pub const MAX_BODY: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub method: String,
    /// The path without its query.
    pub path: String,
    /// `key=value` pairs of the query, percent-decoded.
    pub query: Vec<(String, String)>,
    /// Names lower-cased.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    pub fn param(&self, name: &str) -> Option<&str> {
        self.query.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    /// The token the caller presented: the `X-Token` header, else
    /// `?token=` (an `EventSource` cannot set headers).
    pub fn token(&self) -> Option<&str> {
        self.header("x-token").or_else(|| self.param("token"))
    }
}

/// Why a request was not read: the status to answer with, and the reason.
#[derive(Debug, Clone, PartialEq)]
pub struct Bad(pub u16, pub &'static str);

/// Read one request. `Err(None)`: the caller went away before sending one.
pub fn read_request(reader: &mut impl BufRead) -> Result<Request, Option<Bad>> {
    let mut head = 0usize;
    let mut line = String::new();
    let mut read_line = |line: &mut String| -> Result<(), Option<Bad>> {
        line.clear();
        let n = reader
            .by_ref()
            .take((MAX_HEAD - head + 1) as u64)
            .read_line(line)
            .map_err(|_| None)?;
        head += n;
        if head > MAX_HEAD {
            return Err(Some(Bad(431, "the request's head is too large")));
        }
        Ok(())
    };
    read_line(&mut line)?;
    if line.is_empty() {
        return Err(None);
    }
    let mut parts = line.split_whitespace();
    let (Some(method), Some(target), Some(version)) = (parts.next(), parts.next(), parts.next()) else {
        return Err(Some(Bad(400, "not an HTTP request line")));
    };
    if !version.starts_with("HTTP/1.") {
        return Err(Some(Bad(505, "HTTP/1.x only")));
    }
    let (method, target) = (method.to_owned(), target.to_owned());
    let mut headers = Vec::new();
    loop {
        read_line(&mut line)?;
        let text = line.trim_end_matches(['\r', '\n']);
        if text.is_empty() {
            break;
        }
        if headers.len() == MAX_HEADERS {
            return Err(Some(Bad(431, "too many headers")));
        }
        let Some((name, value)) = text.split_once(':') else {
            return Err(Some(Bad(400, "a header without a colon")));
        };
        headers.push((name.trim().to_ascii_lowercase(), value.trim().to_owned()));
    }
    let length = match headers.iter().find(|(k, _)| k == "content-length") {
        Some((_, v)) => v.parse::<usize>().map_err(|_| Some(Bad(400, "a Content-Length that is not a number")))?,
        None => 0,
    };
    if headers.iter().any(|(k, _)| k == "transfer-encoding") {
        return Err(Some(Bad(411, "send a Content-Length, not a chunked body")));
    }
    if length > MAX_BODY {
        return Err(Some(Bad(413, "the body is too large")));
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).map_err(|_| Some(Bad(400, "the body is shorter than its Content-Length")))?;
    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path.to_owned(), parse_query(query)),
        None => (target, Vec::new()),
    };
    Ok(Request { method, path, query, headers, body })
}

fn parse_query(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            (percent_decode(k), percent_decode(v))
        })
        .collect()
}

/// `%XX` and `+` decoded; anything malformed is kept as it came.
pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() && bytes[i + 1].is_ascii_hexdigit() && bytes[i + 2].is_ascii_hexdigit() => {
                out.push(hex(bytes[i + 1]) << 4 | hex(bytes[i + 2]));
                i += 2;
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        _ => b - b'A' + 10,
    }
}

/// Compare two secrets in time that does not depend on where they differ.
pub fn same_secret(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut diff = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        diff |= usize::from(a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0xff));
    }
    diff == 0
}

pub fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        411 => "Length Required",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        431 => "Request Header Fields Too Large",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        505 => "HTTP Version Not Supported",
        _ => "",
    }
}

/// Write a whole response and close: `Connection: close` always.
pub fn respond(out: &mut impl Write, status: u16, content_type: &str, body: &[u8]) -> std::io::Result<()> {
    respond_with(out, status, content_type, &[], body)
}

/// [`respond`], with headers of the caller's (names and values it controls:
/// no line breaks).
pub fn respond_with(
    out: &mut impl Write,
    status: u16,
    content_type: &str,
    headers: &[(&str, String)],
    body: &[u8],
) -> std::io::Result<()> {
    let mut head = format!(
        "HTTP/1.1 {status} {}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n",
        reason(status),
        body.len()
    );
    for (name, value) in headers {
        debug_assert!(!name.contains(['\r', '\n']) && !value.contains(['\r', '\n']));
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    out.write_all(head.as_bytes())?;
    out.write_all(body)?;
    out.flush()
}

pub fn respond_json(out: &mut impl Write, status: u16, body: &serde_json::Value) -> std::io::Result<()> {
    respond(out, status, "application/json", body.to_string().as_bytes())
}

/// The head of an event stream; the events follow until either side goes.
pub fn start_events(out: &mut impl Write) -> std::io::Result<()> {
    out.write_all(
        b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\n\
          X-Accel-Buffering: no\r\nConnection: close\r\n\r\nretry: 2000\n\n",
    )?;
    out.flush()
}

/// One server-sent event: `data` must be a single line (compact JSON is).
pub fn event(name: &str, data: &str) -> String {
    debug_assert!(!data.contains('\n'));
    format!("event: {name}\ndata: {data}\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(raw: &[u8]) -> Result<Request, Option<Bad>> {
        read_request(&mut std::io::BufReader::new(raw))
    }

    #[test]
    fn a_post_is_read_with_its_body_query_and_headers() {
        let r = parse(
            b"POST /api/call?token=a%20b&x HTTP/1.1\r\nHost: duck\r\nContent-Type: application/json\r\n\
              X-Token: s3cret\r\nContent-Length: 7\r\n\r\n{\"a\":1}trailing",
        )
        .unwrap();
        assert_eq!(r.method, "POST");
        assert_eq!(r.path, "/api/call");
        assert_eq!(r.param("token"), Some("a b"));
        assert_eq!(r.param("x"), Some(""));
        assert_eq!(r.header("content-type"), Some("application/json"));
        assert_eq!(r.token(), Some("s3cret"));
        assert_eq!(r.body, b"{\"a\":1}");
    }

    #[test]
    fn the_query_token_serves_when_no_header_does() {
        let r = parse(b"GET /events?token=t%2B1 HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(r.path, "/events");
        assert_eq!(r.token(), Some("t+1"));
        assert!(r.body.is_empty());
    }

    #[test]
    fn what_is_not_http_or_too_large_is_refused_with_a_status() {
        assert_eq!(parse(b""), Err(None));
        assert_eq!(parse(b"hello\r\n\r\n").unwrap_err().unwrap().0, 400);
        assert_eq!(parse(b"GET / SPDY/3\r\n\r\n").unwrap_err().unwrap().0, 505);
        assert_eq!(parse(b"GET / HTTP/1.1\r\nbroken\r\n\r\n").unwrap_err().unwrap().0, 400);
        let big = format!("POST / HTTP/1.1\r\nContent-Length: {}\r\n\r\n", MAX_BODY + 1);
        assert_eq!(parse(big.as_bytes()).unwrap_err().unwrap().0, 413);
        let chunked = b"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n";
        assert_eq!(parse(chunked).unwrap_err().unwrap().0, 411);
        let long = format!("GET /{} HTTP/1.1\r\n\r\n", "a".repeat(MAX_HEAD));
        assert_eq!(parse(long.as_bytes()).unwrap_err().unwrap().0, 431);
        let many: String = (0..=MAX_HEADERS).map(|i| format!("h{i}: v\r\n")).collect();
        let many = format!("GET / HTTP/1.1\r\n{many}\r\n");
        assert_eq!(parse(many.as_bytes()).unwrap_err().unwrap().0, 431);
        assert_eq!(parse(b"POST / HTTP/1.1\r\nContent-Length: 9\r\n\r\nshort").unwrap_err().unwrap().0, 400);
    }

    #[test]
    fn percent_decoding_keeps_what_it_cannot_read() {
        assert_eq!(percent_decode("a%41+b"), "aA b");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz"), "%zz");
        assert_eq!(percent_decode("%4"), "%4");
    }

    #[test]
    fn secrets_compare_equal_only_when_equal() {
        assert!(same_secret("abc", "abc"));
        assert!(!same_secret("abc", "abd"));
        assert!(!same_secret("abc", "ab"));
        assert!(!same_secret("", "a"));
        assert!(same_secret("", ""));
    }

    #[test]
    fn a_response_says_its_length_and_closes() {
        let mut out = Vec::new();
        respond(&mut out, 404, "text/plain", b"nope").unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.starts_with("HTTP/1.1 404 Not Found\r\n"), "{text}");
        assert!(text.contains("Content-Length: 4\r\n"));
        assert!(text.contains("Connection: close\r\n"));
        assert!(text.ends_with("\r\n\r\nnope"));
        assert_eq!(event("pose", "{\"x\":1}"), "event: pose\ndata: {\"x\":1}\n\n");
        let mut out = Vec::new();
        respond_with(&mut out, 200, "image/jpeg", &[("X-Frame-Age-Ms", "12".into())], &[0xFF, 0xD8]).unwrap();
        let head = String::from_utf8_lossy(&out[..out.len() - 2]).into_owned();
        assert!(head.contains("Content-Type: image/jpeg\r\n") && head.contains("Content-Length: 2\r\n"), "{head}");
        assert!(head.ends_with("X-Frame-Age-Ms: 12\r\n\r\n"), "{head}");
        assert_eq!(&out[out.len() - 2..], &[0xFF, 0xD8]);
    }
}
