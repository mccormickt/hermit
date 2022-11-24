use proxy_wasm::traits::*;
use proxy_wasm::types::*;

proxy_wasm::main! {{
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> { Box::new(HttpBodyRoot) });
}}

struct HttpBodyRoot;

impl Context for HttpBodyRoot {}

impl RootContext for HttpBodyRoot {
    fn get_type(&self) -> Option<ContextType> {
        Some(ContextType::HttpContext)
    }

    fn create_http_context(&self, _: u32) -> Option<Box<dyn HttpContext>> {
        Some(Box::new(HttpBody))
    }

    fn on_configure(&mut self, _plugin_configuration_size: usize) -> bool {
        if let Some(config) = self.get_plugin_configuration() {
            let ips = String::from_utf8(config).unwrap();
            self.set_shared_data(ips.as_str(), Some(b"block"), None);
        }
        false
    }
}

struct HttpBody;

impl Context for HttpBody {}

impl HttpContext for HttpBody {
    fn on_http_request_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        if let Some(remote_address) = self.get_http_response_header("x-forwarded-for") {
            let should_block = self.get_shared_data(remote_address.as_str());
            if should_block {
                self.
            }
        }
        Action::Continue
    }
    fn on_http_response_headers(&mut self, _: usize, _: bool) -> Action {
        // If there is a Content-Length header and we change the length of
        // the body later, then clients will break. So remove it.
        // We must do this here, because once we exit this function we
        // can no longer modify the response headers.
        self.set_http_response_header("content-length", None);
        Action::Continue
    }

    fn on_http_response_body(&mut self, body_size: usize, end_of_stream: bool) -> Action {
        if !end_of_stream {
            // Wait -- we'll be called again when the complete body is buffered
            // at the host side.
            return Action::Pause;
        }

        // Replace the message body if it contains the text "secret".
        // Since we returned "Pause" previuously, this will return the whole body.
        if let Some(body_bytes) = self.get_http_response_body(0, body_size) {
            let body_str = String::from_utf8(body_bytes).unwrap();
            if body_str.contains("secret") {
                let new_body = format!("Original message body ({} bytes) redacted.", body_size);
                self.set_http_response_body(0, body_size, &new_body.into_bytes());
            }
        }
        Action::Continue
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use proxy_wasm_test_framework::tester;
    use proxy_wasm_test_framework::types::{LogLevel, ReturnType};
    use structopt::StructOpt;

    #[test]
    fn test() -> Result<()> {
        let args = tester::MockSettings::from_args();
        let mut test = tester::mock(args)?;

        test.call_start().execute_and_expect(ReturnType::None)?;

        let root_context = 1;
        test.call_proxy_on_context_create(root_context, 0)
            .execute_and_expect(ReturnType::None)?;

        test.call_proxy_on_vm_start(root_context, 0)
            .expect_log(Some(LogLevel::Info), Some("Hello, World!"))
            .expect_set_tick_period_millis(Some(5 * 10u64.pow(3)))
            .execute_and_expect(ReturnType::Bool(true))?;

        test.call_proxy_on_tick(root_context)
            .expect_get_current_time_nanos()
            .returning(Some(0 * 10u64.pow(9)))
            .expect_log(Some(LogLevel::Info), Some("It's 1970-01-01 00:00:00 UTC"))
            .execute_and_expect(ReturnType::None)?;

        test.call_proxy_on_tick(root_context)
            .expect_get_current_time_nanos()
            .returning(None)
            .expect_log(Some(LogLevel::Info), None)
            .execute_and_expect(ReturnType::None)?;

        return Ok(());
    }
}
