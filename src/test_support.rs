//! Helpers shared by unit tests that need a real HTTP peer.

use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

/// Serves `responses` in order, one connection each, on a local port. The join handle
/// yields each raw request, lowercased, so tests can assert what the client sent.
pub(crate) fn serve(responses: Vec<Vec<u8>>) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let server = thread::spawn(move || {
        responses
            .into_iter()
            .map(|response| {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0_u8; 8192];
                let read = stream.read(&mut request).unwrap();
                stream.write_all(&response).unwrap();
                String::from_utf8_lossy(&request[..read]).to_lowercase()
            })
            .collect()
    });
    (address, server)
}

/// A 200 response carrying `body` as JSON, closing the connection afterwards.
pub(crate) fn json_response(body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

/// The body of a raw request returned by [`serve`].
pub(crate) fn request_body(request: &str) -> &str {
    request.split_once("\r\n\r\n").map_or("", |(_, body)| body)
}
