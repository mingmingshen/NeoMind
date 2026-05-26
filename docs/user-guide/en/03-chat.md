# AI Chat

The AI Chat page (path `/` or `/chat`) is the main entry point for NeoMind. You can query device states, control actuators, create automation rules, and analyze data -- all through natural language.

![Chat interface overview](../../img/chat-main.png)

> **Annotated layout**:  **(1)** Header bar with menu button, logo, and user avatar. **(2)** Message history area showing user messages and AI responses. **(3)** Tool call process card (collapsible) showing device queries or command executions. **(4)** Input toolbar with model selector, attachment button, and skill selector. **(5)** Text input field with send button.

---

## Opening the Chat

There are two ways to reach the chat:

1. **Direct navigation**: Go to `/` or `/chat` in your browser. A new session is created automatically if none exists.
2. **Global chat button**: On any other page (Devices, Rules, etc.), click the floating chat button in the bottom-right corner. This opens a full-screen chat overlay that preserves its own session independently.

![Empty chat with welcome](../../img/chat-empty.png)

> **New session state**: When a session has no messages yet, you see a welcome area with suggested prompts. Click any suggestion to populate the input field, or type your own message.

---

## Session Management

### Session History Drawer

Click the **menu** button (hamburger icon) in the top-left of the header to open the session history drawer. The drawer slides in from the left and displays all your past conversations grouped by time: **Today**, **Yesterday**, **This Week**, and **Older**.

Each session entry shows:
- Session title (auto-generated from the first message, or manually renamed)
- Preview of the last message
- Relative timestamp
- Message count

**Search**: Type in the search field at the top of the drawer to filter sessions by title or content.

### Switching Sessions

Click any session in the drawer to switch to it. The chat area loads that session's full message history. The active session is highlighted with a border and accent color.

### Creating a New Session

1. Open the session drawer.
2. Click the **New Conversation** button at the top of the drawer.
3. A blank session opens with the welcome area and suggestion prompts.

Alternatively, on the full chat page, navigating to `/chat` without a session ID auto-creates a new session.

### Deleting a Session

1. Hover over a session in the drawer.
2. Click the **trash** icon that appears on the right.
3. Confirm the deletion in the dialog. The session and all its messages are permanently removed.

---

## Sending a Message

### Text Input

Type your message in the text field at the bottom of the chat area. The input field auto-expands vertically as you type, up to a maximum height.

| Key | Action |
|-----|--------|
| **Enter** | Send the message |
| **Shift+Enter** | Insert a new line without sending |
| **Escape** | Dismiss suggestions panel |
| **/** (at start of empty input) | Open command suggestions |

### Image Attachment

Attach images for the AI to analyze (photos of equipment, screenshots of errors, diagrams, etc.):

1. Click the **image** button in the input toolbar.
2. Select one or more images from your file system.
3. Image thumbnails appear above the input field. Hover and click the X to remove any attachment.
4. Type an optional text prompt, then send.

You can also **drag and drop** image files directly onto the input area. Supported formats: PNG, JPEG, WebP (max 10 MB each).

> **Note**: Image analysis requires a vision-capable model. If the current model does not support images, the attachment button appears disabled. Vision-capable models include `gpt-4o`, `qwen2.5-vl`, `qwen3.5`, `gemini-1.5-flash`, and others.

### Suggested Prompts

Type **/** at the beginning of an empty input to open the suggestions panel. Suggestions are context-aware, showing relevant prompts based on your devices, time of day, and recent activity. Use arrow keys to navigate and Enter to select.

---

## AI Responses

### Streaming Text

AI responses stream in token by token. You see the answer appear in real time as it is generated.

### Thinking Block

For reasoning models (Qwen3, DeepSeek-R1, etc.), the AI's internal reasoning is shown in a **Thinking** block above the response. This block is collapsible -- click the header to expand or collapse it. When the AI performs multiple rounds of reasoning, each round is labeled (R1, R2, ...) with color-coded badges.

### Tool Call Process Card

When the AI executes a tool (querying devices, creating rules, sending commands), a **tool call process card** appears above the response text. The card shows:

- A summary header with the count of tool calls and rounds (e.g., "3 tool calls · 2 steps")
- Per-step tool calls with status indicators: completed (green check), running (yellow spinner), or pending (gray dot)
- Expand each tool call to see its arguments and result as formatted JSON
- Cards with more than 4 tool calls auto-collapse when all are complete

### Stream Progress Bar

During long operations, a progress bar appears at the bottom showing the current stage (Thinking, Collecting, Executing, Generating) and elapsed time.

![AI responding to a message](../../img/chat-typing.png)

> **During streaming**: The input field is disabled and shows a "typing" indicator. A pulsing send button indicates the stream is active. To cancel, click the X button that replaces the send button.

---

## Model Selector

Click the **model selector** button (lightning icon with model name) in the input toolbar to switch LLM backends mid-conversation. Each backend shows:
- A health indicator dot (green if healthy, gray if unavailable)
- Backend name and model identifier
- Backend type (e.g., Ollama, OpenAI-compatible)

The selected model is highlighted with a checkmark. Switching models preserves the full conversation history.

---

## Skill Selector

The **Skills** button (book icon) in the input toolbar opens a dropdown of available skills. Skills provide scenario-specific guides that help the AI execute complex multi-step operations correctly.

To activate skills:
1. Click the **Skills** button in the input toolbar.
2. Select one or more skills from the dropdown.
3. Active skills appear as chips below the toolbar. Click the X on any chip to deselect.
4. When you send a message, the selected skills are included in the AI's context.

Built-in skills cover device onboarding, dashboard management, rule creation, agent management, and more.

---

## Multi-Turn Conversation

The AI maintains full context within a session. You can build complex workflows through natural conversation:

1. "What's the temperature in the greenhouse?"
2. "Is that higher than yesterday?"
3. "Turn on the fan if it exceeds 30 degrees."

Each message builds on previous context -- no need to repeat device names or parameters.

---

## Quick Reference

| Action | How To |
|--------|--------|
| Open session history | Click menu button in header |
| New conversation | Click "New Conversation" in session drawer |
| Switch session | Click any session in the drawer |
| Delete session | Hover, click trash icon, confirm |
| Search sessions | Type in search field in session drawer |
| Attach image | Click image button or drag-drop |
| Switch model | Click model selector in input toolbar |
| Activate skills | Click Skills button, select from dropdown |
| Open suggestions | Type `/` in empty input |
| Cancel stream | Click X button during streaming |

---

[< Back to Settings](./02-settings.md) | [Next: Device Management >](./04-devices.md)
