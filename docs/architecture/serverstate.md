# ServerState 结构设计

> NeoMind v0.4.2
> 创建时间: 2025-02-05

## 当前结构

ServerState 是 Axum handlers 共享的状态容器，包含 25+ 个字段。

### 字段分组

#### 1. 核心服务 (Core Services)
```rust
pub session_manager: Arc<SessionManager>          // 会话管理
pub event_bus: Option<Arc<EventBus>>              // 事件总线
pub command_manager: Option<Arc<CommandManager>>  // 命令管理
pub message_manager: Arc<MessageManager>          // 消息管理
```

#### 2. 设备相关 (Device)
```rust
pub device_registry: Arc<DeviceRegistry>           // 设备注册表
pub device_service: Arc<DeviceService>            // 设备服务
pub time_series_storage: Arc<TimeSeriesStorage>   // 时序数据
#[cfg(feature = "embedded-broker")]
pub embedded_broker: Option<Arc<EmbeddedBroker>>  // 嵌入式 Broker
pub device_update_tx: broadcast::Sender<...>      // 设备状态广播
```

#### 3. 自动化相关 (Automation)
```rust
pub rule_engine: Arc<RuleEngine>                  // 规则引擎
pub rule_store: Option<Arc<RuleStore>>            // 规则存储
pub rule_history_store: Option<Arc<...>>          // 规则历史
pub automation_store: Option<Arc<...>>            // 自动化存储
pub intent_analyzer: Option<Arc<...>>             // 意图分析
pub transform_engine: Option<Arc<...>>            // 转换引擎
pub auto_onboard_manager: Arc<RwLock<Option<...>>> // 自动上线
```

#### 4. Agent 相关 (AI Agent)
```rust
pub memory: Arc<RwLock<TieredMemory>>             // 三层记忆
pub agent_store: Arc<AgentStore>                  // Agent 存储
pub agent_manager: Arc<RwLock<Option<...>>>       // Agent 管理器
```

#### 5. 认证相关 (Auth)
```rust
pub auth_state: Arc<AuthState>                    // API Key 认证
pub auth_user_state: Arc<AuthUserState>           // JWT 认证
```

#### 6. 扩展相关 (Extension)
```rust
pub extension_registry: Arc<RwLock<ExtensionRegistry>> // 扩展注册表
```

#### 7. 跨切面服务 (Cross-cutting)
```rust
pub response_cache: Arc<ResponseCache>            // 响应缓存
pub rate_limiter: Arc<RateLimiter>                // 速率限制
pub dashboard_store: Arc<DashboardStore>          // 仪表板存储
```

#### 8. 内部状态 (Internal)
```rust
pub started_at: i64                                // 启动时间
agent_events_initialized: Arc<AtomicBool>         // 初始化标志
rule_engine_events_initialized: Arc<AtomicBool>
rule_engine_event_service: Arc<Mutex<Option<...>>>
```

---

## 未来重构方向

### 方案 1: 子 State 结构

```rust
pub struct ServerState {
    pub auth: AuthState,
    pub devices: DeviceState,
    pub automation: AutomationState,
    pub agents: AgentState,
    pub core: CoreState,
    pub extensions: ExtensionState,
    pub cross_cutting: CrossCuttingState,
}

#[derive(Clone)]
pub struct AuthState {
    pub api_keys: Arc<AuthState>,
    pub jwt: Arc<AuthUserState>,
}

#[derive(Clone)]
pub struct DeviceState {
    pub registry: Arc<DeviceRegistry>,
    pub service: Arc<DeviceService>,
    pub telemetry: Arc<TimeSeriesStorage>,
    pub embedded_broker: Option<Arc<EmbeddedBroker>>,
    pub update_tx: broadcast::Sender<DeviceStatusUpdate>,
}
```

### 方案 2: 服务定位器模式

```rust
pub struct ServerState {
    services: Arc<ServiceProvider>,
}

pub struct ServiceProvider {
    // 使用 get<T>() 方法获取服务
}

// Handler 使用:
let devices = state.get::<DeviceService>();
```

---

## 注意事项

1. **渐进式迁移**: 不要一次性重构，先创建新结构并保持兼容
2. **Handler 兼容性**: 确保 Handler 代码能平滑过渡
3. **性能考虑**: Arc 包装已经是线程安全，拆分后需保持同样性能
4. **测试覆盖**: 重构前确保有充分的测试

---

## 当前状态

- ✅ 文档化完成
- ⏳ ServerState 拆分待后续迭代
