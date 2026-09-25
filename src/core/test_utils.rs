use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

/*-------------------------------------------------------------------------------------------------
  Test Utilities
-------------------------------------------------------------------------------------------------*/

/// A small, real subset of the AWS IP Ranges JSON (see `tests/fixtures/ip-ranges.json`).
pub(crate) const FIXTURE_JSON: &str = include_str!("../../tests/fixtures/ip-ranges.json");

/*--------------------------------------------------------------------------------------
  Scripted HTTP Server
--------------------------------------------------------------------------------------*/

/// A scripted response for [serve].
pub(crate) enum HttpResponse {
    /// Respond with an HTTP status code and body.
    Respond(u16, String),

    /// Accept the connection and never respond.
    Stall,
}

impl HttpResponse {
    pub(crate) fn ok(body: &str) -> Self {
        Self::Respond(200, body.to_string())
    }

    pub(crate) fn status(code: u16) -> Self {
        Self::Respond(code, String::new())
    }
}

/// Serve the scripted responses, one per connection and in order, on a loopback port and return
/// the URL to request.
pub(crate) fn serve(responses: Vec<HttpResponse>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/ip-ranges.json", listener.local_addr().unwrap());

    thread::spawn(move || {
        for response in responses {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };

            // Read the request headers
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            while reader.read_line(&mut line).is_ok_and(|n| n > 2) {
                line.clear();
            }

            match response {
                HttpResponse::Respond(code, body) => {
                    let _ = write!(
                        stream,
                        "HTTP/1.1 {code} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                }
                HttpResponse::Stall => thread::sleep(Duration::from_secs(30)),
            }
        }
    });

    url
}
