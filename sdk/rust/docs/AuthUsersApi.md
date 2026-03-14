# \AuthUsersApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**add_user_to_tenant**](AuthUsersApi.md#add_user_to_tenant) | **POST** /v1/auth/users/{user_id}/tenants | Add a user to a tenant (grant tenant access)
[**create_user**](AuthUsersApi.md#create_user) | **POST** /auth/v1beta/users | 
[**find_user_by_username**](AuthUsersApi.md#find_user_by_username) | **GET** /v1/auth/users/search/by-username | Find a user by username
[**get_user**](AuthUsersApi.md#get_user) | **GET** /v1/auth/users/{id} | Get a user by ID
[**invite_user**](AuthUsersApi.md#invite_user) | **POST** /v1/auth/users/invite | Invite a user to a tenant
[**list_users**](AuthUsersApi.md#list_users) | **GET** /v1/auth/users | List all users in an operator
[**update_user_role**](AuthUsersApi.md#update_user_role) | **PUT** /v1/auth/users/{user_id}/role | Update a user's role in a specific tenant



## add_user_to_tenant

> models::UserResponse add_user_to_tenant(user_id, add_user_to_tenant_request)
Add a user to a tenant (grant tenant access)

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | User ID | [required] |
**add_user_to_tenant_request** | [**AddUserToTenantRequest**](AddUserToTenantRequest.md) |  | [required] |

### Return type

[**models::UserResponse**](UserResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## create_user

> models::CreateUserResponse create_user(create_user_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**create_user_request** | [**CreateUserRequest**](CreateUserRequest.md) |  | [required] |

### Return type

[**models::CreateUserResponse**](CreateUserResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## find_user_by_username

> models::UserResponse find_user_by_username(username)
Find a user by username

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**username** | **String** | Username to search for | [required] |

### Return type

[**models::UserResponse**](UserResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_user

> models::UserResponse get_user(id)
Get a user by ID

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | User ID | [required] |

### Return type

[**models::UserResponse**](UserResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## invite_user

> models::UserResponse invite_user(invite_user_request)
Invite a user to a tenant

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**invite_user_request** | [**InviteUserRequest**](InviteUserRequest.md) |  | [required] |

### Return type

[**models::UserResponse**](UserResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_users

> models::UserListResponse list_users(operator_id)
List all users in an operator

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**operator_id** | **String** | Operator ID to list users for | [required] |

### Return type

[**models::UserListResponse**](UserListResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## update_user_role

> models::UserResponse update_user_role(user_id, update_user_role_request)
Update a user's role in a specific tenant

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** | User ID | [required] |
**update_user_role_request** | [**UpdateUserRoleRequest**](UpdateUserRoleRequest.md) |  | [required] |

### Return type

[**models::UserResponse**](UserResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

