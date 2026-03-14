# \AuthApiKeysApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_api_key**](AuthApiKeysApi.md#create_api_key) | **POST** /v1/auth/service-accounts/{service_account_id}/api-keys | Create a new API key for a service account
[**list_api_keys**](AuthApiKeysApi.md#list_api_keys) | **GET** /v1/auth/service-accounts/{service_account_id}/api-keys | List API keys for a service account
[**verify_api_key**](AuthApiKeysApi.md#verify_api_key) | **POST** /v1/auth/api-keys/verify | Verify a public API key



## create_api_key

> models::ApiKeyResponse create_api_key(service_account_id, create_api_key_request)
Create a new API key for a service account

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**service_account_id** | **String** | Service account ID | [required] |
**create_api_key_request** | [**CreateApiKeyRequest**](CreateApiKeyRequest.md) |  | [required] |

### Return type

[**models::ApiKeyResponse**](ApiKeyResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_api_keys

> models::ApiKeyListResponse list_api_keys(service_account_id, operator_id)
List API keys for a service account

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**service_account_id** | **String** | Service account ID | [required] |
**operator_id** | **String** | Operator ID | [required] |

### Return type

[**models::ApiKeyListResponse**](ApiKeyListResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## verify_api_key

> models::VerifyApiKeyResponse verify_api_key(verify_api_key_request)
Verify a public API key

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**verify_api_key_request** | [**VerifyApiKeyRequest**](VerifyApiKeyRequest.md) |  | [required] |

### Return type

[**models::VerifyApiKeyResponse**](VerifyApiKeyResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

