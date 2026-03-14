# \AuthVerifyApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**sign_in_with_platform**](AuthVerifyApi.md#sign_in_with_platform) | **POST** /auth/v1beta/sign-in-with-platform | 
[**verify**](AuthVerifyApi.md#verify) | **POST** /auth/v1beta/verify | 



## sign_in_with_platform

> models::SignInWithPlatformResponse sign_in_with_platform(sign_in_with_platform_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**sign_in_with_platform_request** | [**SignInWithPlatformRequest**](SignInWithPlatformRequest.md) |  | [required] |

### Return type

[**models::SignInWithPlatformResponse**](SignInWithPlatformResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## verify

> models::VerifyResponse verify(verify_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**verify_request** | [**VerifyRequest**](VerifyRequest.md) |  | [required] |

### Return type

[**models::VerifyResponse**](VerifyResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

