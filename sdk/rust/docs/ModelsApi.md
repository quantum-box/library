# \ModelsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_models**](ModelsApi.md#get_models) | **GET** /v1/llms/models | Get list of supported models



## get_models

> models::ModelsResponse get_models(x_operator_id, authorization, supported_feature, require_agent_product)
Get list of supported models

Get list of all LLM models available for agents.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**x_operator_id** | **String** | Operator ID | [required] |
**authorization** | **String** | Authorization | [required] |
**supported_feature** | Option<[**Vec<String>**](String.md)> | TODO: add English documentation |  |
**require_agent_product** | Option<**bool**> | TODO: add English documentation |  |

### Return type

[**models::ModelsResponse**](ModelsResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

