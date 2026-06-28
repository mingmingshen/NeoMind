//! run_case orchestration: spawn → seed → inject → turns → queries → teardown.
use crate::{
    case::Case,
    fallback,
    fixture,
    record::{CaseRecord, TurnRecord},
    runtime,
    state_query,
    test_server::TestServer,
};
use anyhow::Result;
use std::sync::Arc;

pub async fn run_case(case_path: &str) -> Result<CaseRecord> {
    let raw = std::fs::read_to_string(case_path)?;
    let case: Case = serde_json::from_str(&raw)?;

    // 1. Spawn per-case test server.
    let server = match TestServer::spawn().await {
        Ok(s) => s,
        Err(e) => return Ok(error_record(&case, "seed_failure", &e.to_string())),
    };

    // 2. Inject agent LLM (panics if env missing).
    let runtime = match runtime::build_agent_runtime_from_env() {
        Ok(r) => r,
        Err(e) => {
            let _ = server.shutdown().await;
            return Ok(error_record(&case, "llm_config_error", &e.to_string()));
        }
    };

    // 3. Point shell tool at this server.
    // SAFETY: eval runs cases sequentially (no parallelism) per spec §6.
    std::env::set_var("NEOMIND_API_BASE", server.api_base());
    std::env::set_var("NEOMIND_API_KEY", server.api_key());

    let result = seed_and_run(&case, &server, runtime).await;

    // 5. Teardown regardless.
    let _ = server.shutdown().await;

    match result {
        Ok(record) => Ok(record),
        Err(e) => Ok(error_record(&case, "runtime_error", &e.to_string())),
    }
}

async fn seed_and_run(
    case: &Case,
    server: &TestServer,
    runtime: Arc<neomind_agent::llm_backends::CloudRuntime>,
) -> Result<CaseRecord> {
    // Seed fixture
    let fix = fixture::load_fixture(format!("eval/fixtures/{}.json", case.setup.fixture))?;
    server.seed_devices(&fix.devices).await?;
    server.seed_metrics(&fix.metrics).await?;
    server.seed_rules(&fix.rules).await?;
    server.seed_agents(&fix.agents).await?;
    server.seed_transforms(&fix.transforms).await?;
    server.seed_dashboards(&fix.dashboards).await?;
    server.seed_channels(&fix.channels).await?;
    server.seed_extensions_metadata(&fix.extensions).await?;

    // Seed case extras
    let x = &case.setup.extras;
    server.seed_devices(&x.devices).await?;
    server.seed_metrics(&x.metrics).await?;
    server.seed_rules(&x.rules).await?;
    server.seed_agents(&x.agents).await?;
    server.seed_transforms(&x.transforms).await?;
    server.seed_dashboards(&x.dashboards).await?;
    server.seed_channels(&x.channels).await?;
    server.seed_extensions_metadata(&x.extensions).await?;

    // Build in-process agent session + inject LLM.
    let sm = neomind_agent::session::SessionManager::memory();
    let sid = sm.create_session().await?;
    sm.get_session(&sid).await?.set_custom_llm(runtime).await;

    // Run turns with per-turn 120s timeout.
    let mut turn_records = Vec::new();
    for turn in &case.turns {
        let resp = match tokio::time::timeout(
            std::time::Duration::from_secs(120),
            sm.process_message(&sid, &turn.user),
        )
        .await
        {
            Ok(r) => r?,
            Err(_) => {
                return Ok(error_record_at(
                    case,
                    "agent_timeout",
                    &format!("turn exceeded 120s: {}", turn.user),
                    turn_records,
                ));
            }
        };
        turn_records.push(TurnRecord {
            user: turn.user.clone(),
            assistant_message: resp.message.content.to_string(),
            tool_calls: resp
                .tool_calls
                .into_iter()
                .map(|tc| {
                    serde_json::json!({
                        "name": tc.name,
                        "arguments": tc.arguments,
                        "result": tc.result,
                    })
                })
                .collect(),
            processing_time_ms: resp.processing_time_ms,
        });
    }

    // State queries
    let state_results = if let Some(qs) = &case.state_queries {
        let mut out = Vec::new();
        for q in qs {
            match state_query::run_query(
                &state_query::StateQueryInput {
                    r#type: q.r#type.clone(),
                    params: q.params.clone(),
                    expected: q.expected.clone(),
                },
                server,
            )
            .await
            {
                Ok(r) => out.push(serde_json::to_value(&r)?),
                Err(e) => out.push(serde_json::json!({
                    "type": q.r#type,
                    "error": e.to_string(),
                    "passed": false
                })),
            }
        }
        out
    } else {
        Vec::new()
    };

    let suspected = fallback::detect_suspected_fallback(&turn_records, &case.expectations.per_turn);

    Ok(CaseRecord {
        case_id: case.id.clone(),
        lang: case.lang.as_str().to_string(),
        turn_records,
        state_queries: state_results,
        suspected_fallback: suspected,
        status: None,
        error_type: None,
        message: None,
    })
}

fn error_record(case: &Case, status: &str, msg: &str) -> CaseRecord {
    error_record_at(case, status, msg, Vec::new())
}

fn error_record_at(case: &Case, status: &str, msg: &str, turn_records: Vec<TurnRecord>) -> CaseRecord {
    CaseRecord {
        case_id: case.id.clone(),
        lang: case.lang.as_str().to_string(),
        turn_records,
        state_queries: Vec::new(),
        suspected_fallback: false,
        status: Some(status.to_string()),
        error_type: Some(status.to_string()),
        message: Some(msg.to_string()),
    }
}
