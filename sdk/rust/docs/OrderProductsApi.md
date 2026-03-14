# \OrderProductsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_product**](OrderProductsApi.md#create_product) | **POST** /v1/order/products | Create a new product
[**delete_product**](OrderProductsApi.md#delete_product) | **DELETE** /v1/order/products/{id} | Delete a product by ID
[**get_product**](OrderProductsApi.md#get_product) | **GET** /v1/order/products/{id} | Get a product by ID
[**list_product_variants**](OrderProductsApi.md#list_product_variants) | **GET** /v1/order/products/{id}/variants | List product variants by product ID
[**list_products**](OrderProductsApi.md#list_products) | **GET** /v1/order/products | List all products with pagination
[**update_product**](OrderProductsApi.md#update_product) | **PUT** /v1/order/products/{id} | Update a product by ID



## create_product

> models::ProductResponse create_product(create_product_request)
Create a new product

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**create_product_request** | [**CreateProductRequest**](CreateProductRequest.md) |  | [required] |

### Return type

[**models::ProductResponse**](ProductResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## delete_product

> models::DeleteProductResponse delete_product(id)
Delete a product by ID

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | Product ID | [required] |

### Return type

[**models::DeleteProductResponse**](DeleteProductResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_product

> models::ProductResponse get_product(id)
Get a product by ID

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | Product ID | [required] |

### Return type

[**models::ProductResponse**](ProductResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_product_variants

> models::ProductVariantListResponse list_product_variants(id)
List product variants by product ID

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | Product ID | [required] |

### Return type

[**models::ProductVariantListResponse**](ProductVariantListResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_products

> models::ProductListResponse list_products(limit, offset)
List all products with pagination

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**limit** | Option<**i32**> | Max items to return |  |
**offset** | Option<**i32**> | Items to skip |  |

### Return type

[**models::ProductListResponse**](ProductListResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## update_product

> models::ProductResponse update_product(id, update_product_request)
Update a product by ID

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | Product ID | [required] |
**update_product_request** | [**UpdateProductRequest**](UpdateProductRequest.md) |  | [required] |

### Return type

[**models::ProductResponse**](ProductResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

