# \AuthUserPoliciesApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**attach_user_policy**](AuthUserPoliciesApi.md#attach_user_policy) | **POST** /v1/auth/user-policies/attach | Attach a policy to a user
[**attach_user_policy_with_scope**](AuthUserPoliciesApi.md#attach_user_policy_with_scope) | **POST** /v1/auth/user-policies/attach-with-scope | Attach a policy to a user with resource scope
[**detach_user_policy**](AuthUserPoliciesApi.md#detach_user_policy) | **POST** /v1/auth/user-policies/detach | Detach a policy from a user
[**detach_user_policy_with_scope**](AuthUserPoliciesApi.md#detach_user_policy_with_scope) | **POST** /v1/auth/user-policies/detach-with-scope | Detach a scoped policy from a user
[**find_user_policy_mappings**](AuthUserPoliciesApi.md#find_user_policy_mappings) | **GET** /v1/auth/user-policy-mappings | Find user policy mappings by resource scope
[**list_user_policies**](AuthUserPoliciesApi.md#list_user_policies) | **GET** /v1/auth/users/{user_id}/policies | List policies attached to a user



## attach_user_policy

> attach_user_policy(attach_user_policy_request)
Attach a policy to a user

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**attach_user_policy_request** | [**AttachUserPolicyRequest**](AttachUserPolicyRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## attach_user_policy_with_scope

> attach_user_policy_with_scope(attach_user_policy_with_scope_request)
Attach a policy to a user with resource scope

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**attach_user_policy_with_scope_request** | [**AttachUserPolicyWithScopeRequest**](AttachUserPolicyWithScopeRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## detach_user_policy

> detach_user_policy(detach_user_policy_request)
Detach a policy from a user

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**detach_user_policy_request** | [**DetachUserPolicyRequest**](DetachUserPolicyRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## detach_user_policy_with_scope

> detach_user_policy_with_scope(detach_user_policy_with_scope_request)
Detach a scoped policy from a user

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**detach_user_policy_with_scope_request** | [**DetachUserPolicyWithScopeRequest**](DetachUserPolicyWithScopeRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## find_user_policy_mappings

> models::UserPolicyMappingListResponse find_user_policy_mappings(tenant_id, resource_scope)
Find user policy mappings by resource scope

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**tenant_id** | **String** | Tenant ID | [required] |
**resource_scope** | **String** | Resource scope in TRN format | [required] |

### Return type

[**models::UserPolicyMappingListResponse**](UserPolicyMappingListResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_user_policies

> models::UserPolicyListResponse list_user_policies(user_id, tenant_id)
List policies attached to a user

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | User ID | [required] |
**tenant_id** | Option<**String**> | Optional tenant ID filter |  |

### Return type

[**models::UserPolicyListResponse**](UserPolicyListResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

