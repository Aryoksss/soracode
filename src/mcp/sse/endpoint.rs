//! SSE handshake: read the POST endpoint the server announces.

use std::time::Duration;

use futures_util::StreamExt;

use crate::error::{Error, Result};

/// Read SSE lines until a POST endpoint is known. Handles the `endpoint`
/// event and the bare first-`data`-is-sessionId convention.
pub(super) async fn read_endpoint<S>(name: &str, url: &str, stream: &mut S) -> Result<String>
where
    S: futures_util::Stream<Item = reqwest::Result<bytes::Bytes>> + Unpin,
{
    let mut buf = String::new();
    let mut event = String::new();
    loop {
        let chunk = tokio::time::timeout(Duration::from_secs(15), stream.next()).await;
        let Some(chunk) = chunk.ok().flatten() else {
            return Err(Error::Tool(format!(
                "mcp `{name}`: `{url}` closed before announcing an endpoint"
            )));
        };
        let bytes = chunk.map_err(|e| Error::Tool(format!("mcp `{name}`: stream: {e}")))?;
        buf.push_str(&String::from_utf8_lossy(&bytes));
        while let Some(pos) = buf.find('\n') {
            let line: String = buf.drain(..=pos).collect();
            let line = line.trim_end();
            if let Some(ev) = line.strip_prefix("event:") {
                event = ev.trim().to_string();
            } else if let Some(data) = line.strip_prefix("data:") {
                let data = data.trim();
                if data.starts_with('{') {
                    continue;
                }
                if event == "endpoint" || data.starts_with('/') || data.starts_with("http") {
                    return Ok(absolute(url, data));
                }
                return Ok(format!("{}/{}", url.trim_end_matches('/'), data));
            }
        }
    }
}

/// Resolve a possibly-relative endpoint path against the base URL origin.
fn absolute(base: &str, path: &str) -> String {
    if path.starts_with("http://") || path.starts_with("https://") {
        return path.to_string();
    }
    let scheme_end = base.find("://").map_or(0, |i| i + 3);
    let host_end = base[scheme_end..]
        .find('/')
        .map_or(base.len(), |i| scheme_end + i);
    format!("{}{}", &base[..host_end], path)
}

#[cfg(test)]
mod tests {
    use super::absolute;

    #[test]
    fn absolute_joins_a_relative_path_onto_the_origin() {
        assert_eq!(
            absolute("http://localhost:9876", "/abc/message"),
            "http://localhost:9876/abc/message"
        );
    }

    #[test]
    fn absolute_drops_a_base_path() {
        assert_eq!(absolute("http://host:1/prefix", "/x"), "http://host:1/x");
    }

    #[test]
    fn absolute_keeps_an_absolute_endpoint() {
        assert_eq!(
            absolute("http://localhost:9876", "http://host:1/x"),
            "http://host:1/x"
        );
    }
}
