# \ChatApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**chat_completion**](ChatApi.md#chat_completion) | **POST** /v1/llms/chat/completions | Create a chat completion
[**chat_completion_on_chatroom**](ChatApi.md#chat_completion_on_chatroom) | **POST** /v1/llms/chatrooms/{chatroom_id}/chat/completions | Create a chat completion within a chatroom



## chat_completion

> models::ChatCompletionChunkResponse chat_completion(x_operator_id, authorization, chat_completion_request)
Create a chat completion

Creates a model response for the given chat conversation.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**x_operator_id** | **String** | Operator ID | [required] |
**authorization** | **String** | Authorization | [required] |
**chat_completion_request** | [**ChatCompletionRequest**](ChatCompletionRequest.md) |  | [required] |

### Return type

[**models::ChatCompletionChunkResponse**](ChatCompletionChunkResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## chat_completion_on_chatroom

> models::ChatCompletionWithChatroomStreamResponse chat_completion_on_chatroom(chatroom_id, x_operator_id, authorization, chat_completion_request)
Create a chat completion within a chatroom

Creates a model response for the given chat conversation in a specific chatroom.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**chatroom_id** | **String** | Chatroom ID | [required] |
**x_operator_id** | **String** | Operator ID | [required] |
**authorization** | **String** | Authorization | [required] |
**chat_completion_request** | [**ChatCompletionRequest**](ChatCompletionRequest.md) |  | [required] |

### Return type

[**models::ChatCompletionWithChatroomStreamResponse**](ChatCompletionWithChatroomStreamResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

