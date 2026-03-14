# \OauthApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**callback**](OauthApi.md#callback) | **GET** /oauth/{provider_name}/callback | OAuth callback handler for specified provider
[**connect**](OauthApi.md#connect) | **GET** /oauth/{provider_name}/connect | Get OAuth authorization URL for specified provider



## callback

> models::OAuthCallbackResponse callback(provider_name, code, state)
OAuth callback handler for specified provider

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**provider_name** | **String** | OAuth provider name | [required] |
**code** | **String** | Authorization code from provider | [required] |
**state** | **String** | State parameter for CSRF protection | [required] |

### Return type

[**models::OAuthCallbackResponse**](OAuthCallbackResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## connect

> models::AuthUrlResponse connect(provider_name)
Get OAuth authorization URL for specified provider

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**provider_name** | **String** | OAuth provider name | [required] |

### Return type

[**models::AuthUrlResponse**](AuthUrlResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

