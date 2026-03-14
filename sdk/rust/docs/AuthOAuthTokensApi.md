# \AuthOAuthTokensApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**delete_oauth_token**](AuthOAuthTokensApi.md#delete_oauth_token) | **DELETE** /v1/auth/oauth-tokens/{provider} | Delete an OAuth token
[**get_oauth_token_by_provider**](AuthOAuthTokensApi.md#get_oauth_token_by_provider) | **GET** /v1/auth/oauth-tokens/{provider} | Get an OAuth token by provider
[**list_oauth_tokens**](AuthOAuthTokensApi.md#list_oauth_tokens) | **GET** /v1/auth/oauth-tokens | List all OAuth tokens
[**save_oauth_token**](AuthOAuthTokensApi.md#save_oauth_token) | **POST** /v1/auth/oauth-tokens | Save an OAuth token



## delete_oauth_token

> delete_oauth_token(provider)
Delete an OAuth token

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**provider** | **String** | OAuth provider name | [required] |

### Return type

 (empty response body)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_oauth_token_by_provider

> models::OAuthTokenDetailResponse get_oauth_token_by_provider(provider)
Get an OAuth token by provider

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**provider** | **String** | OAuth provider name | [required] |

### Return type

[**models::OAuthTokenDetailResponse**](OAuthTokenDetailResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_oauth_tokens

> models::OAuthTokenListResponse list_oauth_tokens()
List all OAuth tokens

### Parameters

This endpoint does not need any parameter.

### Return type

[**models::OAuthTokenListResponse**](OAuthTokenListResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## save_oauth_token

> models::OAuthTokenResponse save_oauth_token(save_o_auth_token_request)
Save an OAuth token

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**save_o_auth_token_request** | [**SaveOAuthTokenRequest**](SaveOAuthTokenRequest.md) |  | [required] |

### Return type

[**models::OAuthTokenResponse**](OAuthTokenResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

