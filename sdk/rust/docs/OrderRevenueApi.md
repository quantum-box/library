# \OrderRevenueApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**calculate_recurring_revenue**](OrderRevenueApi.md#calculate_recurring_revenue) | **POST** /v1/order/revenue/recurring | Calculate recurring revenue for a given period



## calculate_recurring_revenue

> models::RecurringRevenueResponse calculate_recurring_revenue(recurring_revenue_request)
Calculate recurring revenue for a given period

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**recurring_revenue_request** | [**RecurringRevenueRequest**](RecurringRevenueRequest.md) |  | [required] |

### Return type

[**models::RecurringRevenueResponse**](RecurringRevenueResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

