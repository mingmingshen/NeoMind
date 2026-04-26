//! Memory Extraction Integration Tests
//!
//! Tests memory extraction with a real LLM (Ollama).
//! All tests are #[ignore] — run with:
//!   cargo test -p neomind-agent --test memory_extraction_integration -- --ignored

use std::sync::Arc;
use tokio::sync::RwLock;

use neomind_agent::{ExtractionConfig, MemoryExtractor, OllamaConfig, OllamaRuntime};
use neomind_core::llm::backend::LlmRuntime;
use neomind_storage::{MarkdownMemoryStore, MemoryCategory, SessionMessage};

fn ollama_available() -> bool {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 11434));
    std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(2)).is_ok()
}

/// Create a temp memory store + extractor backed by Ollama glm5.
async fn create_extractor() -> (
    tempfile::TempDir,
    Arc<RwLock<MarkdownMemoryStore>>,
    MemoryExtractor,
) {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let store = MarkdownMemoryStore::new(temp_dir.path().to_path_buf());
    store.init().unwrap();
    let store = Arc::new(RwLock::new(store));

    let config = OllamaConfig {
        endpoint: "http://localhost:11434".to_string(),
        model: "qwen3.5:4b".to_string(),
        timeout_secs: 120,
    };
    let llm: Arc<dyn LlmRuntime> = Arc::new(OllamaRuntime::new(config).unwrap());

    let extraction_config = ExtractionConfig {
        min_messages: 1,
        max_messages: 50,
        min_importance: 20,
        dedup_enabled: true,
        similarity_threshold: 0.85,
    };

    let extractor = MemoryExtractor::with_config(store.clone(), llm, extraction_config);

    (temp_dir, store, extractor)
}

// ========== Test 1: Agent extraction should produce all 4 categories ==========

#[tokio::test]
#[ignore = "Requires Ollama with qwen3.5:4b. Run: cargo test -p neomind-agent --test memory_extraction_integration -- --ignored"]
async fn test_agent_extraction_produces_all_categories() -> anyhow::Result<()> {
    if !ollama_available() {
        eprintln!("Ollama not available, skipping");
        return Ok(());
    }

    let (_temp_dir, store, extractor) = create_extractor().await;

    let reasoning_steps = r#"
1. [analyze] 读取温度传感器，当前仓库温度 27.5°C
2. [alert] 温度超过 25°C 阈值，但历史数据显示此温度在该时段（14:00-15:00）为正常日照升温
3. [command] 不发送告警。经过多次执行发现 25°C 阈值在午后导致大量误报，建议调整到 28°C
4. [analyze] 传感器 #3 读数比其他传感器高约 2°C，可能存在校准偏差
"#;

    let conclusion = r#"
执行完成。当前温度 27.5°C 未触发告警。
发现：25°C 的告警阈值太低，在下午时段会产生大量误报，建议提升至 28°C。
传感器 #3 存在约 +2°C 的系统性偏差，后续读数需要补偿。
通过双次确认策略，已将误报率降低约 80%。
"#;

    let count = extractor
        .extract_from_agent(
            "温控卫士",
            Some("监控仓库温度，超过阈值时告警"),
            reasoning_steps,
            conclusion,
        )
        .await?;

    println!("Extracted {} memory entries", count);
    assert!(count > 0, "Should extract at least one memory");

    // Check each category
    let store_guard = store.read().await;
    for category in MemoryCategory::all() {
        let content = store_guard.read_category(&category).unwrap_or_default();
        println!("\n[{:?}] ->\n{}", category, content);
    }

    let up = store_guard
        .read_category(&MemoryCategory::UserProfile)
        .unwrap_or_default();
    let dk = store_guard
        .read_category(&MemoryCategory::DomainKnowledge)
        .unwrap_or_default();
    let tp = store_guard
        .read_category(&MemoryCategory::TaskPatterns)
        .unwrap_or_default();
    let se = store_guard
        .read_category(&MemoryCategory::SystemEvolution)
        .unwrap_or_default();

    // At least some categories should have content
    let non_empty = [&up, &dk, &tp, &se]
        .iter()
        .filter(|c| !c.trim().is_empty())
        .count();
    assert!(
        non_empty >= 2,
        "At least 2 categories should be populated, got {}",
        non_empty
    );

    // Key assertion: system_evolution must NOT be empty
    assert!(
        !se.trim().is_empty(),
        "system_evolution must have content — the agent discovered threshold and baseline insights.\n\
         Got: {:?}",
        se
    );

    Ok(())
}

// ========== Test 2: Chat extraction should skip system_evolution ==========

#[tokio::test]
#[ignore = "Requires Ollama with qwen3.5:4b. Run: cargo test -p neomind-agent --test memory_extraction_integration -- --ignored"]
async fn test_chat_extraction_skips_system_evolution() -> anyhow::Result<()> {
    if !ollama_available() {
        eprintln!("Ollama not available, skipping");
        return Ok(());
    }

    let (_temp_dir, store, extractor) = create_extractor().await;

    let messages = vec![
        SessionMessage::user("你好，我想了解一下温度监控系统"),
        SessionMessage::assistant(
            "你好！温度监控系统可以帮你实时监控环境温度。你有什么具体需求吗？",
        ),
        SessionMessage::user("我喜欢用中文交流，另外我的仓库里有三个温度传感器"),
        SessionMessage::assistant(
            "好的，已记录你使用中文，仓库有三个温度传感器。需要设置告警阈值吗？",
        ),
        SessionMessage::user("是的，设定温度超过30度就告警，用微信通知我"),
        SessionMessage::assistant(
            "好的，已设置温度告警阈值 30°C，通知方式为微信。还有什么需要吗？",
        ),
    ];

    let count = extractor.extract_from_chat(&messages).await?;

    println!("Chat extracted {} memory entries", count);

    let store_guard = store.read().await;
    for category in MemoryCategory::all() {
        let content = store_guard.read_category(&category).unwrap_or_default();
        println!("\n[{:?}] ->\n{}", category, content);
    }

    let se = store_guard
        .read_category(&MemoryCategory::SystemEvolution)
        .unwrap_or_default();

    // Chat extraction should NEVER produce system_evolution entries
    // The file may contain header/metadata but no actual entries (lines starting with "- ")
    let se_has_entries = se.lines().any(|line| line.trim().starts_with("- ["));
    assert!(
        !se_has_entries,
        "system_evolution should have no entries after chat extraction, but got: {:?}",
        se
    );

    // At least user_profile or domain_knowledge should have something
    let up = store_guard
        .read_category(&MemoryCategory::UserProfile)
        .unwrap_or_default();
    let dk = store_guard
        .read_category(&MemoryCategory::DomainKnowledge)
        .unwrap_or_default();
    let tp = store_guard
        .read_category(&MemoryCategory::TaskPatterns)
        .unwrap_or_default();

    let non_se_non_empty = [&up, &dk, &tp]
        .iter()
        .filter(|c| !c.trim().is_empty())
        .count();
    assert!(
        non_se_non_empty >= 1,
        "At least 1 non-system_evolution category should be populated"
    );

    Ok(())
}

// ========== Test 3: Full pipeline — extract → snapshot ==========

#[tokio::test]
#[ignore = "Requires Ollama with qwen3.5:4b. Run: cargo test -p neomind-agent --test memory_extraction_integration -- --ignored"]
async fn test_full_pipeline_extract_and_snapshot() -> anyhow::Result<()> {
    if !ollama_available() {
        eprintln!("Ollama not available, skipping");
        return Ok(());
    }

    let (_temp_dir, store, extractor) = create_extractor().await;

    let reasoning_steps = r#"
1. [analyze] 检查湿度传感器，当前湿度 85%
2. [alert] 湿度超过 80% 阈值，存在结露风险
3. [command] 发送告警通知用户，建议开启除湿设备
4. [analysis] 发现湿度在每天凌晨 3-5 点会规律性升高，可能与通风系统关闭有关
"#;

    let conclusion = r#"
湿度告警已发送。发现规律：凌晨 3-5 点湿度会升至 80% 以上，
原因是通风系统定时关闭。建议调整通风时间表或降低告警阈值。
"#;

    // Step 1: Extract
    let count = extractor
        .extract_from_agent(
            "环境监控助手",
            Some("监控仓库湿度，超过阈值告警"),
            reasoning_steps,
            conclusion,
        )
        .await?;

    println!("Extracted {} memories", count);
    assert!(count > 0, "Should extract memories");

    // Step 2: Load snapshot
    let store_guard = store.read().await;
    let snapshot = neomind_agent::memory::snapshot::MemorySnapshot::load(&store_guard);

    println!("\nSnapshot content:\n{}", snapshot.to_prompt_section());

    assert!(
        !snapshot.is_empty(),
        "Snapshot should have content after extraction"
    );

    let section = snapshot.to_prompt_section();
    assert!(
        section.contains("<memory-context>"),
        "Snapshot should contain memory-context tag"
    );

    // The snapshot should contain at least some of the extracted memory content
    // (can't assert exact content since LLM output varies)
    assert!(
        section.len() > 50,
        "Snapshot should have meaningful content"
    );

    Ok(())
}
