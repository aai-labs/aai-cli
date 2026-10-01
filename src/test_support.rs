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
