# \DeliverySoftwareApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_software_delivery_by_order**](DeliverySoftwareApi.md#get_software_delivery_by_order) | **GET** /v1/delivery/software/by-order/{order_id} | Get software delivery by order ID



## get_software_delivery_by_order

> models::SoftwareDeliveryResponse get_software_delivery_by_order(order_id)
Get software delivery by order ID

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**order_id** | **String** | Purchase order ID | [required] |

### Return type

[**models::SoftwareDeliveryResponse**](SoftwareDeliveryResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

