wit_bindgen::generate!({
    generate_all
});

use anyhow::{anyhow, bail, ensure, Result};
use chrono::prelude::*;
use exports::wasi::http::incoming_handler::Guest;
use wasi::http::types::{*};
use wasi::logging::logging::*;
use wasmcloud::messaging::*;
use crate::wasi::io::streams::StreamError;

struct HttpServer;

const MAX_READ_BYTES: u32 = 2048;
const MAX_WRITE_BYTES: usize = 4096;

impl Guest for HttpServer {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        let method = request.method().to_string();

        let path_with_query = request
            .path_with_query()
            .map(String::from)
            .unwrap_or_else(|| "/".into());

        let (path, query) = path_with_query
            .split_once('?')
            .unwrap_or(("/", ""));

        let body_data = request
            .read_body()
            .expect("failed to read request body");

        log(Level::Info, "", &format!("Received body: {:?}", body_data));

        
        let msg_json = serde_json::json!({
            "method": method,
            "path": path,
            "query": query,
            "body": body_data,
            "timestamp": Utc::now().timestamp_millis(),
        });
        log(Level::Info, "", &format!("Read body: {}", msg_json));

        let msg_data = serde_json::to_vec(&msg_json)
            .expect("failed to serialize message");

        let msg = types::BrokerMessage{
            subject: "http".to_string(),
            reply_to: None,
            body: msg_data,
        };
        consumer::publish(&msg);
        log(Level::Info, "", &format!("Published message: {:?}", msg));

        let response = OutgoingResponse::new(Fields::new());
        response.set_status_code(200).unwrap();

        let response_body = response.body().unwrap();
        ResponseOutparam::set(response_out, Ok(response));
        response_body
            .write()
            .unwrap()
            .blocking_write_and_flush(b"Successfully written to message bus")
            .unwrap();
        OutgoingBody::finish(response_body, None).expect("failed to finish response body");
    }
}

impl IncomingRequest {
    /// This is a convenience function that writes out the body of a IncomingRequest (from wasi:http)
    /// into anything that supports [`std::io::Write`]
    fn read_body(self) -> Result<Vec<u8>> {
        // Read the body
        let incoming_req_body = self
            .consume()
            .map_err(|()| anyhow!("failed to consume incoming request body"))?;
        let incoming_req_body_stream = incoming_req_body
            .stream()
            .map_err(|()| anyhow!("failed to build stream for incoming request body"))?;
        let mut buf = Vec::<u8>::with_capacity(MAX_READ_BYTES as usize);
        loop {
            match incoming_req_body_stream.read(MAX_READ_BYTES as u64) {
                Ok(bytes) if bytes.is_empty() => break,
                Ok(bytes) => {
                    ensure!(
                        bytes.len() <= MAX_READ_BYTES as usize,
                        "read more bytes than requested"
                    );
                    buf.extend(bytes);
                }
                Err(StreamError::Closed) => break,
                Err(e) => bail!("failed to read bytes: {e}"),
            }
        }
        buf.shrink_to_fit();
        drop(incoming_req_body_stream);
        IncomingBody::finish(incoming_req_body);
        Ok(buf)
    }
}

impl ToString for Method {
    fn to_string(&self) -> String {
        match self {
            Method::Get => "GET".into(),
            Method::Post => "POST".into(),
            Method::Patch => "PATCH".into(),
            Method::Put => "PUT".into(),
            Method::Delete => "DELETE".into(),
            Method::Options => "OPTIONS".into(),
            Method::Head => "HEAD".into(),
            Method::Connect => "CONNECT".into(),
            Method::Trace => "TRACE".into(),
            Method::Other(m) => m.into(),
        }
    }
}

export!(HttpServer);
