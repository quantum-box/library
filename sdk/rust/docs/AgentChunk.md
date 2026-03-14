# AgentChunk

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**is_client_tool** | Option<**bool**> | When `true`, this tool call targets a client-defined tool. The client should handle it locally and submit the result via the tool-result endpoint (unless `fire_and_forget`). | [optional]
**tool_id** | **String** |  | 
**tool_name** | **String** | Name of the client-defined tool the LLM wants to call. | 
**r#type** | **Type** |  (enum: tool_job_started) | 
**args** | Option<**serde_json::Value**> |  | 
**is_finished** | **bool** |  | 
**result** | **String** |  | 
**fire_and_forget** | Option<**bool**> | When `true`, the server does not wait for the client to submit a tool result — the LLM continues immediately. The client may still execute the tool for its side effects. | [optional]
**index** | **i32** |  | 
**text** | **String** |  | 
**created_at** | **String** |  | 
**id** | **String** |  | 
**user_id** | **String** |  | 
**options** | **Vec<String>** |  | 
**command** | Option<**String**> |  | [optional]
**cache_creation_input_tokens** | Option<**i32**> |  | [optional]
**cache_read_input_tokens** | Option<**i32**> |  | [optional]
**completion_tokens** | **i32** |  | 
**prompt_tokens** | **i32** |  | 
**total_cost** | Option<**f64**> |  | [optional]
**total_tokens** | **i32** |  | 
**job_id** | **String** |  | 
**provider** | **String** |  | 
**agent** | Option<[**models::AgentSource**](AgentSource.md)> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


