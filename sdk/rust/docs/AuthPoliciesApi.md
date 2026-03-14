# \AuthPoliciesApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**check_policy_for_resource**](AuthPoliciesApi.md#check_policy_for_resource) | **POST** /v1/auth/policies/check-for-resource | Check permission for a specific resource
[**evaluate_policies_batch**](AuthPoliciesApi.md#evaluate_policies_batch) | **POST** /v1/auth/policies/check | Evaluate multiple policy actions in batch
[**get_policy**](AuthPoliciesApi.md#get_policy) | **GET** /v1/auth/policies/{id} | Get a policy by ID
[**list_actions**](AuthPoliciesApi.md#list_actions) | **GET** /v1/auth/actions | List all registered actions



## check_policy_for_resource

> models::CheckPolicyForResourceResponse check_policy_for_resource(check_policy_for_resource_request)
Check permission for a specific resource

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**check_policy_for_resource_request** | [**CheckPolicyForResourceRequest**](CheckPolicyForResourceRequest.md) |  | [required] |

### Return type

[**models::CheckPolicyForResourceResponse**](CheckPolicyForResourceResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## evaluate_policies_batch

> models::EvaluatePoliciesBatchResponse evaluate_policies_batch(evaluate_policies_batch_request)
Evaluate multiple policy actions in batch

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**evaluate_policies_batch_request** | [**EvaluatePoliciesBatchRequest**](EvaluatePoliciesBatchRequest.md) |  | [required] |

### Return type

[**models::EvaluatePoliciesBatchResponse**](EvaluatePoliciesBatchResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_policy

> models::PolicyResponse get_policy(id)
Get a policy by ID

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | Policy ID | [required] |

### Return type

[**models::PolicyResponse**](PolicyResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_actions

> models::ActionListResponse list_actions(context)
List all registered actions

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**context** | Option<**String**> | Filter by context |  |

### Return type

[**models::ActionListResponse**](ActionListResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

