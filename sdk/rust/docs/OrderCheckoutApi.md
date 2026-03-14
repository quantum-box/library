# \OrderCheckoutApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**process_checkout**](OrderCheckoutApi.md#process_checkout) | **POST** /v1/order/checkout | Process payment checkout for a quote



## process_checkout

> models::CheckoutResponse process_checkout(checkout_request)
Process payment checkout for a quote

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**checkout_request** | [**CheckoutRequest**](CheckoutRequest.md) |  | [required] |

### Return type

[**models::CheckoutResponse**](CheckoutResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

