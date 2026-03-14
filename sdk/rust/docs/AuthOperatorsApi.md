# \AuthOperatorsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_operator**](AuthOperatorsApi.md#create_operator) | **POST** /v1/auth/operators | Create an operator under a platform
[**find_operators_by_user**](AuthOperatorsApi.md#find_operators_by_user) | **GET** /v1/auth/operators/by-user | Find operators accessible to a user under a platform
[**get_operator_by_alias**](AuthOperatorsApi.md#get_operator_by_alias) | **GET** /v1/auth/operators/by-alias | Get an operator by alias within a platform
[**get_operator_by_id**](AuthOperatorsApi.md#get_operator_by_id) | **GET** /v1/auth/operators/{id} | Get an operator by ID



## create_operator

> models::CreateOperatorResponse create_operator(create_operator_request)
Create an operator under a platform

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**create_operator_request** | [**CreateOperatorRequest**](CreateOperatorRequest.md) |  | [required] |

### Return type

[**models::CreateOperatorResponse**](CreateOperatorResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## find_operators_by_user

> models::OperatorListResponse find_operators_by_user(platform_id, user_id)
Find operators accessible to a user under a platform

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**platform_id** | **String** | Platform ID | [required] |
**user_id** | **String** | User ID | [required] |

### Return type

[**models::OperatorListResponse**](OperatorListResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_operator_by_alias

> models::OperatorResponse get_operator_by_alias(platform_id, alias)
Get an operator by alias within a platform

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**platform_id** | **String** | Platform ID | [required] |
**alias** | **String** | Operator alias (username) | [required] |

### Return type

[**models::OperatorResponse**](OperatorResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_operator_by_id

> models::OperatorResponse get_operator_by_id(id)
Get an operator by ID

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | Operator ID | [required] |

### Return type

[**models::OperatorResponse**](OperatorResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

