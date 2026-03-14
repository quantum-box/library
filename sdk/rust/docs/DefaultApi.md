# \DefaultApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_chatroom_messages**](DefaultApi.md#get_chatroom_messages) | **GET** /v1/llms/chatrooms/{chatroom_id}/messages | Get messages from a chatroom
[**get_chatrooms**](DefaultApi.md#get_chatrooms) | **GET** /v1/llms/chatrooms | Get chatrooms list



## get_chatroom_messages

> models::ChatroomsChatroomIdMessagesGetResponse get_chatroom_messages(chatroom_id, limit, offset, search, x_operator_id, authorization)
Get messages from a chatroom

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**chatroom_id** | **String** | Chatroom ID | [required] |
**limit** | **i32** | Limit | [required] |
**offset** | **i32** | Offset | [required] |
**search** | **String** | Search | [required] |
**x_operator_id** | **String** | Operator ID | [required] |
**authorization** | **String** | Authorization | [required] |

### Return type

[**models::ChatroomsChatroomIdMessagesGetResponse**](ChatroomsChatroomIdMessagesGetResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_chatrooms

> models::GetChatroomsResponse get_chatrooms(x_operator_id, authorization, limit, offset, search, metadata_user_id)
Get chatrooms list

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**x_operator_id** | **String** | Operator ID | [required] |
**authorization** | **String** | Authorization | [required] |
**limit** | Option<**i32**> | Maximum number of chatrooms to return (default: 20, max: 100) |  |
**offset** | Option<**i32**> | Number of chatrooms to skip for pagination |  |
**search** | Option<**String**> | Search chatrooms by name (partial match) |  |
**metadata_user_id** | Option<**String**> | Filter chatrooms by `user_id` stored in metadata JSON. Use this to retrieve chatrooms associated with a specific external user (e.g., LINE user ID). Only chatrooms where `metadata.user_id` matches exactly will be returned. |  |

### Return type

[**models::GetChatroomsResponse**](GetChatroomsResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

