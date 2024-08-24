wit_bindgen::generate!({ generate_all });

use exports::wasmcloud::messaging::handler::Guest;
use wasi::logging::logging::*;
use wasmcloud::messaging::*;

struct Validator;

impl Guest for Validator {
    fn handle_message(msg: types::BrokerMessage) -> Result<(), String> {
        let msg = types::BrokerMessage {
            subject: "eventdriven.valid".to_string(),
            reply_to: None,
            body: "Event received: TODO validate".as_bytes().to_vec(),
        };
        consumer::publish(&msg);
        log(Level::Info, "", "Received message");
        Ok(())
    }
}

export!(Validator);
