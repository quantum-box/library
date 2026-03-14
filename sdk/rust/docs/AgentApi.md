# \AgentApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**execute_agent**](AgentApi.md#execute_agent) | **POST** /v1/llms/chatrooms/{chatroom_id}/agent/execute | Execute agent
[**get_agent_messages**](AgentApi.md#get_agent_messages) | **GET** /v1/llms/chatrooms/{chatroom_id}/agent/messages | Get agent message log
[**get_agent_status**](AgentApi.md#get_agent_status) | **GET** /v1/llms/chatrooms/{chatroom_id}/agent/status | Get agent status



## execute_agent

> models::AgentChunk execute_agent(chatroom_id, x_operator_id, authorization, agent_execute_request)
Execute agent

Execute a task using the agent system. Returns an SSE stream of `AgentChunk` events.  ## Client-Side Tools  You can provide custom tool definitions via the `client_tools` field. When the LLM decides to call one of these tools, the server emits a `tool_call_pending` SSE event containing the tool name, arguments, and a unique `tool_id`. The client should:  1. Execute the tool locally using the provided arguments 2. Submit the result via    `POST /v1/llms/chatrooms/{chatroom_id}/agent/tool-result`    with the matching `tool_id` 3. The agent resumes processing with the tool result  For **fire-and-forget** tools (`fire_and_forget: true`), the server immediately returns a success message to the LLM without waiting for a result submission. The client still receives the `tool_call_pending` event and can execute the tool, but does not need to submit a result.  Tool names must not collide with server built-in tools (e.g. `read_file`, `execute_command`). The server validates this and returns an error if a collision is detected.  When `client_tools` is provided, JSON tool call mode is automatically enabled — you do not need to set `use_json_tool_calls` separately.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**chatroom_id** | **String** | Chatroom ID | [required] |
**x_operator_id** | **String** | Operator ID | [required] |
**authorization** | **String** | Authorization | [required] |
**agent_execute_request** | [**AgentExecuteRequest**](AgentExecuteRequest.md) |  | [required] |

### Return type

[**models::AgentChunk**](AgentChunk.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: text/event-stream

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_agent_messages

> models::AgentMessagesResponse get_agent_messages(chatroom_id, x_operator_id, authorization, limit, offset)
Get agent message log

Get agent message log for specified chatroom.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**chatroom_id** | **String** | チャットルームID | [required] |
**x_operator_id** | **String** | Operator ID | [required] |
**authorization** | **String** | Authorization | [required] |
**limit** | Option<**i32**> | 取得するメッセージの最大数 |  |
**offset** | Option<**i32**> | 取得開始位置 |  |

### Return type

[**models::AgentMessagesResponse**](AgentMessagesResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_agent_status

> models::AgentStatusResponse get_agent_status(chatroom_id, x_operator_id, authorization)
Get agent status

Get execution status of agent for specified chatroom.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**chatroom_id** | **String** | Chatroom ID | [required] |
**x_operator_id** | **String** | Operator ID | [required] |
**authorization** | **String** | Authorization | [required] |

### Return type

[**models::AgentStatusResponse**](AgentStatusResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

