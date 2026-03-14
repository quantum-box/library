# \OrderQuotesApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_quote**](OrderQuotesApi.md#create_quote) | **POST** /v1/order/quotes | Create a new quote
[**get_quote**](OrderQuotesApi.md#get_quote) | **GET** /v1/order/quotes/{id} | Get a quote by ID
[**issue_quote**](OrderQuotesApi.md#issue_quote) | **POST** /v1/order/quotes/{id}/issue | Issue a quote to a client
[**list_quotes**](OrderQuotesApi.md#list_quotes) | **GET** /v1/order/quotes | List all quotes
[**update_quote**](OrderQuotesApi.md#update_quote) | **PUT** /v1/order/quotes/{id} | Update a quote by ID



## create_quote

> models::QuoteResponse create_quote(create_quote_request)
Create a new quote

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**create_quote_request** | [**CreateQuoteRequest**](CreateQuoteRequest.md) |  | [required] |

### Return type

[**models::QuoteResponse**](QuoteResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_quote

> models::QuoteResponse get_quote(id)
Get a quote by ID

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | Quote ID | [required] |

### Return type

[**models::QuoteResponse**](QuoteResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## issue_quote

> models::QuoteResponse issue_quote(id, issue_quote_request)
Issue a quote to a client

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | Quote ID | [required] |
**issue_quote_request** | [**IssueQuoteRequest**](IssueQuoteRequest.md) |  | [required] |

### Return type

[**models::QuoteResponse**](QuoteResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_quotes

> models::QuoteListResponse list_quotes()
List all quotes

### Parameters

This endpoint does not need any parameter.

### Return type

[**models::QuoteListResponse**](QuoteListResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## update_quote

> models::QuoteResponse update_quote(id, update_quote_request)
Update a quote by ID

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | Quote ID | [required] |
**update_quote_request** | [**UpdateQuoteRequest**](UpdateQuoteRequest.md) |  | [required] |

### Return type

[**models::QuoteResponse**](QuoteResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

