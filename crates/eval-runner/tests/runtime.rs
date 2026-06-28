use eval_runner::runtime::build_agent_runtime_from_env;
use std::env;

#[test]
#[should_panic(expected = "AGENT_LLM_API_KEY required")]
fn missing_env_panics() {
    env::remove_var("AGENT_LLM_API_KEY");
    env::remove_var("AGENT_LLM_ENDPOINT");
    env::remove_var("AGENT_LLM_MODEL");
    let _ = build_agent_runtime_from_env(); // should panic
}
