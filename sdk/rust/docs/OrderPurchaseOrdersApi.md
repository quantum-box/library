# \OrderPurchaseOrdersApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_purchase_order**](OrderPurchaseOrdersApi.md#get_purchase_order) | **GET** /v1/order/purchase-orders/{id} | Get a purchase order by ID
[**list_purchase_orders**](OrderPurchaseOrdersApi.md#list_purchase_orders) | **GET** /v1/order/purchase-orders | List all purchase orders



## get_purchase_order

> models::PurchaseOrderResponse get_purchase_order(id)
Get a purchase order by ID

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | Purchase order ID | [required] |

### Return type

[**models::PurchaseOrderResponse**](PurchaseOrderResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_purchase_orders

> models::PurchaseOrderListResponse list_purchase_orders()
List all purchase orders

### Parameters

This endpoint does not need any parameter.

### Return type

[**models::PurchaseOrderListResponse**](PurchaseOrderListResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

