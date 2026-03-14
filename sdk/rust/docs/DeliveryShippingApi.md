# \DeliveryShippingApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**check_shipping_availability**](DeliveryShippingApi.md#check_shipping_availability) | **GET** /v1/delivery/shipping-destinations/{id}/availability | Check physical shipping availability
[**create_shipping_destination**](DeliveryShippingApi.md#create_shipping_destination) | **POST** /v1/delivery/shipping-destinations | Create a shipping destination



## check_shipping_availability

> models::ShippingAvailabilityResponse check_shipping_availability(id)
Check physical shipping availability

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | Shipping destination ID | [required] |

### Return type

[**models::ShippingAvailabilityResponse**](ShippingAvailabilityResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## create_shipping_destination

> models::ShippingDestinationResponse create_shipping_destination(create_shipping_destination_request)
Create a shipping destination

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**create_shipping_destination_request** | [**CreateShippingDestinationRequest**](CreateShippingDestinationRequest.md) |  | [required] |

### Return type

[**models::ShippingDestinationResponse**](ShippingDestinationResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

