# \AuthServiceAccountsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_service_account**](AuthServiceAccountsApi.md#create_service_account) | **POST** /v1/auth/service-accounts | Create a new service account
[**delete_service_account**](AuthServiceAccountsApi.md#delete_service_account) | **DELETE** /v1/auth/service-accounts/{id} | Delete a service account
[**get_service_account**](AuthServiceAccountsApi.md#get_service_account) | **GET** /v1/auth/service-accounts/{id} | Get a service account by ID
[**list_service_accounts**](AuthServiceAccountsApi.md#list_service_accounts) | **GET** /v1/auth/service-accounts | List all service accounts
[**update_service_account**](AuthServiceAccountsApi.md#update_service_account) | **PUT** /v1/auth/service-accounts/{id} | Update a service account



## create_service_account

> models::ServiceAccountResponse create_service_account(create_service_account_request)
Create a new service account

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**create_service_account_request** | [**CreateServiceAccountRequest**](CreateServiceAccountRequest.md) |  | [required] |

### Return type

[**models::ServiceAccountResponse**](ServiceAccountResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## delete_service_account

> models::DeleteServiceAccountResponse delete_service_account(id)
Delete a service account

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | Service account ID | [required] |

### Return type

[**models::DeleteServiceAccountResponse**](DeleteServiceAccountResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_service_account

> models::ServiceAccountResponse get_service_account(id, operator_id)
Get a service account by ID

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | Service account ID | [required] |
**operator_id** | **String** | Operator ID | [required] |

### Return type

[**models::ServiceAccountResponse**](ServiceAccountResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_service_accounts

> models::ServiceAccountListResponse list_service_accounts(operator_id)
List all service accounts

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**operator_id** | **String** | Operator ID | [required] |

### Return type

[**models::ServiceAccountListResponse**](ServiceAccountListResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## update_service_account

> models::ServiceAccountResponse update_service_account(id, update_service_account_request)
Update a service account

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | Service account ID | [required] |
**update_service_account_request** | [**UpdateServiceAccountRequest**](UpdateServiceAccountRequest.md) |  | [required] |

### Return type

[**models::ServiceAccountResponse**](ServiceAccountResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

