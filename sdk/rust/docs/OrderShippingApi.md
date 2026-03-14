# \OrderShippingApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**register_shipping_destination**](OrderShippingApi.md#register_shipping_destination) | **POST** /v1/order/shipping-destinations | Register a shipping destination for a quote



## register_shipping_destination

> models::ShippingDestinationResponse register_shipping_destination(register_shipping_destination_request)
Register a shipping destination for a quote

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**register_shipping_destination_request** | [**RegisterShippingDestinationRequest**](RegisterShippingDestinationRequest.md) |  | [required] |

### Return type

[**models::ShippingDestinationResponse**](ShippingDestinationResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

