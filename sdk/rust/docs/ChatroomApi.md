# \ChatroomApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_chatroom**](ChatroomApi.md#create_chatroom) | **POST** /v1/llms/chatrooms | Create a new chatroom
[**delete_chatroom**](ChatroomApi.md#delete_chatroom) | **DELETE** /v1/llms/chatrooms/{chatroom_id} | Delete a chatroom
[**update_chatroom**](ChatroomApi.md#update_chatroom) | **PATCH** /v1/llms/chatrooms/{chatroom_id} | Update a chatroom



## create_chatroom

> models::CreateChatRoomResponse create_chatroom(x_operator_id, authorization, create_chat_room_request)
Create a new chatroom

Creates a new chatroom for the authenticated user.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**x_operator_id** | **String** | Operator ID | [required] |
**authorization** | **String** | Authorization | [required] |
**create_chat_room_request** | [**CreateChatRoomRequest**](CreateChatRoomRequest.md) |  | [required] |

### Return type

[**models::CreateChatRoomResponse**](CreateChatRoomResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## delete_chatroom

> delete_chatroom(chatroom_id, x_operator_id, authorization)
Delete a chatroom

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**chatroom_id** | **String** | Chatroom ID | [required] |
**x_operator_id** | **String** | Operator ID | [required] |
**authorization** | **String** | Authorization | [required] |

### Return type

 (empty response body)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## update_chatroom

> models::UpdateChatroomResponse update_chatroom(chatroom_id, x_operator_id, authorization, update_chatroom_request)
Update a chatroom

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**chatroom_id** | **String** | Chatroom ID | [required] |
**x_operator_id** | **String** | Operator ID | [required] |
**authorization** | **String** | Authorization | [required] |
**update_chatroom_request** | [**UpdateChatroomRequest**](UpdateChatroomRequest.md) |  | [required] |

### Return type

[**models::UpdateChatroomResponse**](UpdateChatroomResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

