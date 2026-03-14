# \PaymentApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_or_create_stripe_customer**](PaymentApi.md#get_or_create_stripe_customer) | **POST** /v1/payment/stripe-customer | Get or create a Stripe customer
[**list_providers**](PaymentApi.md#list_providers) | **GET** /v1/payment/providers | List payment providers by entity ID



## get_or_create_stripe_customer

> models::StripeCustomerResponse get_or_create_stripe_customer(stripe_customer_request)
Get or create a Stripe customer

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**stripe_customer_request** | [**StripeCustomerRequest**](StripeCustomerRequest.md) |  | [required] |

### Return type

[**models::StripeCustomerResponse**](StripeCustomerResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_providers

> models::ProviderListResponse list_providers(entity_id)
List payment providers by entity ID

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**entity_id** | **String** | Entity ID | [required] |

### Return type

[**models::ProviderListResponse**](ProviderListResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

