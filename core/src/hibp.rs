use sha1::{Digest, Sha1};
#[cfg(test)]
use std::io::{Read, Write};
#[cfg(test)]
use std::net::TcpListener;

#[derive(Debug)]
pub enum HibpError {
    Request,
}

pub const HIBP_BASE_URL: &str = "https://api.pwnedpasswords.com";

pub fn sha1_hex_upper(password: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(password.as_bytes());
    hasher.finalize().iter().map(|b| format!("{:02X}", b)).collect()
}

fn parse_range_response(body: &str) -> Vec<(String, u32)> {
    body.lines()
        .filter_map(|line| {
            let (suffix, count) = line.trim().split_once(':')?;
            count.trim().parse::<u32>().ok().map(|c| (suffix.to_string(), c))
        })
        .collect()
}

/// Checks one password against HIBP's Pwned-Passwords k-anonymity range API. `base_url` is
/// injectable so tests point this at a local mock; only a truncated 5-char hash prefix ever leaves
/// this function in the URL, never the password itself.
///
/// Synchronous (`reqwest::blocking`): nestkey-core stays framework-free, so it cannot use Tauri's
/// `async_runtime::block_on` the way the pre-M6 src-tauri copy did. Callers that run inside a tokio
/// runtime (nestkey-web) must invoke `do_check_breach` via `spawn_blocking`, since `reqwest::blocking`
/// panics if dropped inside an async context.
pub fn check_password(
    client: &reqwest::blocking::Client,
    base_url: &str,
    password: &str,
) -> Result<Option<u32>, HibpError> {
    let full_hash = sha1_hex_upper(password);
    let prefix = &full_hash[..5];
    let url = format!("{base_url}/range/{prefix}");
    let response = client.get(&url).send().map_err(|_| HibpError::Request)?;
    let body = response.text().map_err(|_| HibpError::Request)?;
    Ok(crate::health::hibp_match(&full_hash, &parse_range_response(&body)))
}

/// Spins up a one-shot local HTTP server returning `response_body` for exactly one request.
/// Returns "http://127.0.0.1:PORT" to point check_password at.
#[cfg(test)]
pub fn mock_hibp_server(response_body: String) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf); // drain the request; contents unused, fixed-response mock
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    format!("http://{}", addr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha1_hex_upper_matches_a_known_test_vector() {
        assert_eq!(sha1_hex_upper("password"), "5BAA61E4C9B93F3F0682250B6CF8331B7EE68FD8");
    }

    #[test]
    fn parse_range_response_handles_multiple_lines() {
        let body = "AAAA:5\r\nBBBB:10\r\n";
        assert_eq!(parse_range_response(body), vec![("AAAA".to_string(), 5), ("BBBB".to_string(), 10)]);
    }

    #[test]
    fn check_password_finds_a_breached_password() {
        let full_hash = sha1_hex_upper("password");
        let suffix = &full_hash[5..];
        let body = format!("{}:12345\r\nFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF:1\r\n", suffix);
        let base_url = mock_hibp_server(body);
        let client = reqwest::blocking::Client::new();

        let result = check_password(&client, &base_url, "password").unwrap();

        assert_eq!(result, Some(12345));
    }

    #[test]
    fn check_password_returns_none_for_an_unbreached_password() {
        let base_url = mock_hibp_server("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF:1\r\n".to_string());
        let client = reqwest::blocking::Client::new();

        let result = check_password(&client, &base_url, "a-totally-unique-password-xyz-123").unwrap();

        assert_eq!(result, None);
    }
}
