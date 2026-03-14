# ToolCallPending

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**args** | Option<**serde_json::Value**> |  | 
**fire_and_forget** | Option<**bool**> | When `true`, the server does not wait for the client to submit a tool result — the LLM continues immediately. The client may still execute the tool for its side effects. | [optional]
**tool_id** | **String** | Unique identifier for this tool call. Use this value as `tool_id` when submitting the result. | 
**tool_name** | **String** | Name of the client-defined tool the LLM wants to call. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


