use std::str;
use std::time::Duration;

use log::info;
use proxy_wasm::hostcalls;
use proxy_wasm::traits::*;
use proxy_wasm::types::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Hermit {
    blocked_ips: Vec<String>,
}

impl Hermit {
    fn new() -> Self {
        return Self {
            blocked_ips: Vec::new(),
        };
    }

    fn get_source_address(&self) -> String {
        // Retrieve source address from properties
        let bytes: Vec<u8> = self.get_property(vec!["source", "address"]).unwrap();
        match str::from_utf8(&bytes) {
            Ok(a) => a,
            Err(_) => "",
        }
        .split(":")
        .take(1)
        .map(|x| x.to_owned())
        .collect()
    }
}

#[no_mangle]
pub fn _start() {
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_stream_context(|_, _| -> Box<dyn StreamContext> { Box::new(Hermit::new()) });
}

impl Context for Hermit {}

impl RootContext for Hermit {
    fn on_vm_start(&mut self, _vm_configuration_size: usize) -> bool {
        hostcalls::log(LogLevel::Debug, "Hermit VM instantiated");
        self.set_tick_period(Duration::from_secs(2));
        true
    }

    fn on_configure(&mut self, _plugin_configuration_size: usize) -> bool {
        let data: Vec<u8> = self.get_plugin_configuration().unwrap();
        self.blocked_ips = serde_json::from_slice::<Hermit>(&data).unwrap().blocked_ips;
        true
    }
}

impl StreamContext for Hermit {
    fn on_new_connection(&mut self) -> Action {
        // Retrieve source address from properties
        let addr = self.get_source_address();

        info!("Recieved connection from: {}", addr);
        info!("Blocked IPs: {}", self.blocked_ips.join(", "));

        // Check if IP is in the block list
        if self.blocked_ips.contains(&addr) {
            info!("Rejected connection from blocked IP: {}", addr);
            self.close_downstream();
        }
        Action::Continue
    }
}

impl HttpContext for Hermit {}

#[cfg(test)]
mod test {}
