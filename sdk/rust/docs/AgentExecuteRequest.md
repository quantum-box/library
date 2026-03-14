# AgentExecuteRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**additional_tool_description** | Option<**String**> | Additional description appended to each tool's help text shown to the LLM. | [optional]
**agent_protocol_id** | Option<**String**> | Optional AgentProtocol ID to use for this execution. | [optional]
**agent_protocol_mode** | Option<[**models::AgentProtocolMode**](AgentProtocolMode.md)> | AgentProtocol selection mode. | [optional]
**assistant_name** | Option<**String**> | Display name for the assistant in the conversation. | [optional]
**auto_approve** | Option<**bool**> | When true, the agent auto-approves tool executions without asking for confirmation. | [optional]
**chatroom_name_generation** | Option<[**models::ChatroomNameGeneration**](ChatroomNameGeneration.md)> | Controls chatroom name auto-generation behavior. - `first_only` (default): Generate on first message only (regardless of current name) - `always`: Always attempt to generate/update name after each message - `never`: Never auto-generate chatroom name | [optional]
**client_tools** | Option<[**Vec<models::ClientToolDefinition>**](ClientToolDefinition.md)> | User-defined tools executed on the client side.  Each tool must have a unique `name` that does not collide with server built-in tool names. When the LLM calls one of these tools, the server emits a `tool_call_pending` SSE event and blocks until the client submits the result via `POST /v1/llms/chatrooms/{chatroom_id}/agent/tool-result` (timeout: 5 minutes).  Tools with `fire_and_forget: true` do not block — the server immediately returns a success to the LLM and the client can execute the tool asynchronously.  Example: ```json {   \"client_tools\": [     {       \"name\": \"send_slack_message\",       \"description\": \"Send a message to a Slack channel\",       \"parameters\": {         \"type\": \"object\",         \"properties\": {           \"channel\": { \"type\": \"string\" },           \"text\": { \"type\": \"string\" }         },         \"required\": [\"channel\", \"text\"]       },       \"fire_and_forget\": false     }   ] } ``` | [optional]
**max_requests** | Option<**i32**> | Maximum number of LLM round-trips (tool calls) the agent is allowed to make before stopping. | [optional]
**mcp_hub_config_json** | Option<**String**> | Optional MCP Hub configuration in JSON string form. | [optional]
**model** | Option<**String**> | Model identifier. Accepts `provider/model` or just `model` (provider auto-detected).  Examples: - `anthropic/claude-3-sonnet-20241022` - `openai/gpt-4` - `google_ai/gemini-pro`  Auto-detection shortcuts: - `gpt-*` → OpenAI - `claude-*` → Anthropic - `gemini*` → Google AI | [optional]
**task** | **String** | The task or prompt for the agent to execute. | 
**tool_access** | Option<[**models::AgentToolAccessRequest**](AgentToolAccessRequest.md)> | Per-category tool access flags. | [optional]
**use_json_tool_calls** | Option<**bool**> | When true, use JSON Schema tool definitions (function calling) instead of XML-based tool parsing. This is automatically enabled when `client_tools` is provided. | [optional]
**user_custom_instructions** | Option<**String**> | Custom system-level instructions appended to the agent's prompt. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


