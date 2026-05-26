# Settings

Settings is the control center for AI model connections, device connections, interface preferences, and data management. Access it from the sidebar navigation or go directly to `/settings`.

The page is organized into four tabs: **LLM Backends**, **Device Connections**, **Preferences**, and **About**. Switch between them using the tab bar at the top.

---

## LLM Backends

The LLM Backends tab manages your AI model connections. NeoMind supports running multiple providers simultaneously -- a local Ollama for fast private tasks and a cloud model for complex reasoning, for example.

![LLM Backends tab](../../img/settings-llm-page.png)

1. **Provider card grid** -- each card represents a supported provider (Ollama, OpenAI, Anthropic, etc.) showing its name, status, and number of configured instances.
2. **Active badge** -- the provider card with a green border and "Running" status indicates the currently active backend.
3. **Click a card** to enter the detail view for that provider.

### Supported Providers

| Provider     | API Key | Default Endpoint                                       | Notes                             |
|-------------|---------|--------------------------------------------------------|-----------------------------------|
| **Ollama**   | No      | `http://localhost:11434`                               | Local-first; uses `/api/chat`     |
| **llama.cpp** | No     | `http://127.0.0.1:8080`                                | Direct llama-server integration   |
| **OpenAI**   | Yes     | `https://api.openai.com/v1`                            | Full function calling support     |
| **Anthropic** | Yes    | `https://api.anthropic.com/v1`                         | Thinking mode supported           |
| **Google**   | Yes     | `https://generativelanguage.googleapis.com/v1beta`      | Vision and thinking supported     |
| **xAI**      | Yes     | `https://api.x.ai/v1`                                  | Grok models                       |
| **Qwen**     | Yes     | `https://dashscope.aliyuncs.com/compatible-mode/v1`     | Alibaba Cloud; vision supported   |
| **DeepSeek** | Yes     | `https://api.deepseek.com/v1`                          | Reasoning models available        |
| **GLM**      | Yes     | `https://open.bigmodel.cn/api/paas/v4`                  | Zhipu AI; vision supported        |
| **MiniMax**  | Yes     | `https://api.minimax.chat/v1`                          | Vision supported                  |

### Adding a Backend

1. Go to **Settings > LLM Backends**.
2. Click the provider card you want to configure (e.g., **Ollama**).
3. In the detail view, click **Add Instance**.
4. Fill in the configuration dialog:
   - **Name** -- a label like "Local Ollama" or "GPT-4o Production".
   - **Endpoint** -- API base URL. Pre-filled with the provider default.
   - **Model** -- model identifier, e.g., `qwen3.5:4b`, `gpt-4o-mini`.
   - **API Key** -- required for cloud providers. Not needed for Ollama/llama.cpp.
   - **Temperature**, **Top P**, **Top K** -- optional sampling parameters.
5. Click **Test Connection** to verify reachability. A success message shows the response latency.
6. Click **Save**.

The new instance becomes the active backend automatically.

### Testing and Managing Instances

Each configured instance appears as a card in the provider detail view. The card shows the instance name, model, endpoint, and test result.

- **Test button** (test tube icon) -- sends a lightweight request to verify the endpoint is reachable, the API key is valid, and the model is available.
- **Edit button** (pencil icon) -- opens the config dialog to modify endpoint, model, or API key.
- **Delete button** (trash icon) -- removes the instance after confirmation.

### Connection Troubleshooting

If the connection test fails:

1. **Ollama**: Is the service running? Run `ollama serve` and check port 11434.
2. **Cloud providers**: Is the API key valid and not expired? Does the endpoint include `/v1`?
3. **llama.cpp**: Is the server started? Run `llama-server -m model.gguf`.
4. **Network**: For cloud providers, ensure outbound HTTPS is allowed.

---

## Device Connections

The Device Connections tab configures how IoT devices connect to NeoMind. Two adapter types are available: **MQTT** (for streaming telemetry) and **Webhook** (for HTTP push).

For detailed device connection setup, see [Device Connections](./04a-device-connection.md).

### MQTT

Click the **MQTT** card to see the built-in broker and any external brokers you have added. The built-in broker runs on port 1883 by default.

- **Add Connection** to register an external MQTT broker with custom address, port, credentials, and TLS settings.
- **Test** each broker to verify connectivity.

### Webhook

Click the **Webhook** card to view the webhook URL template and usage instructions. Devices push data via HTTP POST -- no persistent connection is needed. Copy the webhook URL and replace `{device_id}` with your device ID.

---

## Preferences

The Preferences tab controls language, time display, and data cleanup policies.

![Preferences tab](../../img/settings-general-page.png)

1. **Language** -- switch between English and Chinese. Changes apply immediately after clicking **Save Settings**.
2. **Time Format** -- choose 12-hour or 24-hour display.
3. **System Timezone** -- select your IANA timezone. This is used for dashboard charts, rule schedules, and log timestamps.
4. **Current time preview** -- shows the current time in the selected timezone and format for verification.

After changing language or time format, click **Save Settings** to apply. To discard changes, click **Reset**.

### Data Management

The Data Management card within Preferences controls automatic data cleanup:

![Data Management](../../img/settings-general-page.png)

1. **Auto Cleanup** toggle -- enable or disable the automatic cleanup task.
2. **Default Retention** -- how long device telemetry data is stored. Options range from 12 hours to 90 days, or "Never" to keep data indefinitely.
3. **Image Retention** -- how long uploaded images (chat, dashboard) are kept. Independent of telemetry retention.
4. **Cleanup Now** button -- manually trigger an immediate cleanup of expired data.

| Retention Period | Best For |
|-----------------|----------|
| 12 hours - 3 days | Resource-constrained devices, high-frequency sensors |
| 7 - 30 days | General use, weekly trend analysis |
| 90 days | Long-term analysis, rare event sensors |
| Never | Archival; monitor disk usage manually |

> **Tip**: For long-term storage, use the Data Push feature to forward telemetry to external databases (InfluxDB, TimescaleDB) while keeping local retention short.

---

## About

The About tab displays system information and version details. It shows:

- **System Info** -- platform, architecture, CPU cores, GPU (if detected), memory usage with a visual bar.
- **Project Info** -- NeoMind version, license (Apache-2.0), and a link to the GitHub repository.
- **Check for Updates** -- available in the desktop (Tauri) app. Click to check if a newer version is available.

---

[< Back to Installation](./01-installation.md) | [Next: AI Chat >](./03-chat.md)
