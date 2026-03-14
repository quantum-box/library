# \OrderClientsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_client**](OrderClientsApi.md#create_client) | **POST** /v1/order/clients | Create a new client
[**get_client**](OrderClientsApi.md#get_client) | **GET** /v1/order/clients/{id} | Get a client by ID
[**list_clients**](OrderClientsApi.md#list_clients) | **GET** /v1/order/clients | List all clients



## create_client

> models::ClientResponse create_client(create_client_request)
Create a new client

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**create_client_request** | [**CreateClientRequest**](CreateClientRequest.md) |  | [required] |

### Return type

[**models::ClientResponse**](ClientResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_client

> models::ClientResponse get_client(id)
Get a client by ID

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | Client ID | [required] |

### Return type

[**models::ClientResponse**](ClientResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_clients

> models::ClientListResponse list_clients()
List all clients

### Parameters

This endpoint does not need any parameter.

### Return type

[**models::ClientListResponse**](ClientListResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

