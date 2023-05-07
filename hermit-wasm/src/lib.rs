use std::str;
use std::time::Duration;

use log::debug;
use log::info;
use proxy_wasm::hostcalls;
use proxy_wasm::traits::*;
use proxy_wasm::types::*;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
struct Config {
    blocked_ips: Option<Vec<String>>,
    blocked_user_agents: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct Hermit {
    #[serde(skip)]
    context_id: u32,
    config: Config,
}

impl Hermit {
    fn new(context_id: u32) -> Self {
        return Self {
            context_id: context_id,
            config: Config {
                blocked_ips: Some(Vec::new()),
                blocked_user_agents: Some(Vec::new()),
            },
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

proxy_wasm::main!({
    {
        proxy_wasm::set_log_level(LogLevel::Trace);
        proxy_wasm::set_root_context(|context_id| -> Box<dyn RootContext> {
            Box::new(Hermit::new(context_id))
        });
    }
});

impl Context for Hermit {}

impl RootContext for Hermit {
    fn on_vm_start(&mut self, _vm_configuration_size: usize) -> bool {
        hostcalls::log(LogLevel::Debug, "Hermit VM instantiated").unwrap();
        self.set_tick_period(Duration::from_secs(2));
        true
    }

    fn on_configure(&mut self, _plugin_configuration_size: usize) -> bool {
        let data: Vec<u8> = self.get_plugin_configuration().unwrap();
        match serde_json::from_slice::<Config>(&data) {
            Ok(c) => {
                self.config = c;
                true
            }
            Err(e) => {
                debug!("data: {:?}", &data);
                info!("couldn't configure wasm plugin: {}", e);
                false
            }
        }
    }

    fn create_stream_context(&self, context_id: u32) -> Option<Box<dyn StreamContext>> {
        Some(Box::new(Hermit {
            context_id: context_id,
            config: self.config.clone(),
        }))
    }

    fn create_http_context(&self, context_id: u32) -> Option<Box<dyn HttpContext>> {
        Some(Box::new(Hermit {
            context_id: context_id,
            config: self.config.clone(),
        }))
    }

    #[cfg(feature = "http")]
    fn get_type(&self) -> Option<ContextType> {
        Some(ContextType::HttpContext)
    }

    #[cfg(feature = "tcp")]
    fn get_type(&self) -> Option<ContextType> {
        Some(ContextType::StreamContext)
    }
}

impl StreamContext for Hermit {
    fn on_new_connection(&mut self) -> Action {
        // Retrieve source address from properties
        let addr = self.get_source_address();
        info!("Recieved connection from: {}", addr);

        // Check if IP is in the block list
        let blocked_ips = self.config.clone().blocked_ips.unwrap();
        if blocked_ips.contains(&addr) {
            info!("Rejected connection from blocked IP: {}", addr);
            self.close_downstream();
        }
        Action::Continue
    }
}

impl HttpContext for Hermit {
    fn on_http_request_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        let header = self
            .get_http_request_header("User-Agent")
            .unwrap_or_default();
        info!("Recieved connection with User-Agent: {}", header);

        // Check if User Agent is in the block list
        let blocked_user_agents = self.config.clone().blocked_user_agents.unwrap();
        if blocked_user_agents.contains(&header) {
            info!("Rejected connection from blocked User-Agent: {}", header);
            self.send_http_response(403, Vec::new(), None)
        }
        Action::Continue
    }
}

#[cfg(test)]
mod test {}
