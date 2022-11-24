#[cfg(test)]
mod test {
    use anyhow::Result;
    use proxy_wasm_test_framework::tester;
    use proxy_wasm_test_framework::types::{LogLevel, ReturnType};
    use structopt::StructOpt;

    #[test]
    fn test() -> Result<()> {
        let args = tester::MockSettings::from_args();
        let mut hello_world_test = tester::mock(args)?;

        hello_world_test
            .call_start()
            .execute_and_expect(ReturnType::None)?;

        let root_context = 1;
        hello_world_test
            .call_proxy_on_context_create(root_context, 0)
            .execute_and_expect(ReturnType::None)?;

        hello_world_test
            .call_proxy_on_vm_start(root_context, 0)
            .expect_log(Some(LogLevel::Info), Some("Hello, World!"))
            .expect_set_tick_period_millis(Some(5 * 10u64.pow(3)))
            .execute_and_expect(ReturnType::Bool(true))?;

        hello_world_test
            .call_proxy_on_tick(root_context)
            .expect_get_current_time_nanos()
            .returning(Some(0 * 10u64.pow(9)))
            .expect_log(Some(LogLevel::Info), Some("It's 1970-01-01 00:00:00 UTC"))
            .execute_and_expect(ReturnType::None)?;

        hello_world_test
            .call_proxy_on_tick(root_context)
            .expect_get_current_time_nanos()
            .returning(None)
            .expect_log(Some(LogLevel::Info), None)
            .execute_and_expect(ReturnType::None)?;

        return Ok(());
    }
}