#![feature(addr_parse_ascii)]
use std::str;
use std::time::Duration;

use log::info;
use proxy_wasm::traits::*;
use proxy_wasm::types::*;

struct Hermit {
    blocked_ips: Vec<String>,
}

impl Hermit {
    fn new() -> Self {
        return Self {
            // Test blocking docker network IP
            blocked_ips: vec!["172.19.0.1".to_string()],
        };
    }
}

#[no_mangle]
pub fn _start() {
    proxy_wasm::set_log_level(LogLevel::Info);
    proxy_wasm::set_stream_context(|_, _| -> Box<dyn StreamContext> { Box::new(Hermit::new()) });
}

impl Context for Hermit {}

impl RootContext for Hermit {
    fn on_vm_start(&mut self, _vm_configuration_size: usize) -> bool {
        info!(
            "Hermit VM instantiated. Blocking IPs: {}",
            self.blocked_ips.join(", "),
        );
        self.set_tick_period(Duration::from_secs(2));
        true
    }
}

impl StreamContext for Hermit {
    fn on_new_connection(&mut self) -> Action {
        // Retrieve source address from properties
        let bytes: Vec<u8> = self.get_property(vec!["source", "address"]).unwrap();
        let addr: String = match str::from_utf8(&bytes) {
            Ok(a) => a,
            Err(_) => "",
        }
        .split(":")
        .take(1)
        .map(|x| x.to_owned())
        .collect();
        info!("Recieved connection from: {}", addr);

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
