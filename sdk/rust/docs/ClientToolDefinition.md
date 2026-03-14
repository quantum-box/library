# ClientToolDefinition

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**description** | **String** | Human-readable description shown to the LLM so it can decide when to call this tool. | 
**fire_and_forget** | Option<**bool**> | When `true`, the server dispatches the tool call to the client and immediately returns a success message to the LLM without waiting for a result submission. The client still receives the `tool_call_pending` SSE event but does not need to call the tool-result endpoint. Useful for notifications, webhooks, logging, and other side-effect-only operations. | [optional]
**name** | **String** | Unique tool name. Must not collide with server built-in tools (e.g. `read_file`, `execute_command`, `search_files`). Use descriptive, snake_case names. | 
**parameters** | Option<**serde_json::Value**> |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


