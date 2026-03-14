# \ProcurementApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**delete_variant_link**](ProcurementApi.md#delete_variant_link) | **DELETE** /v1/procurement/variant-links/{id} | Delete a variant procurement link
[**upsert_variant_link**](ProcurementApi.md#upsert_variant_link) | **POST** /v1/procurement/variant-links | Upsert a variant procurement link



## delete_variant_link

> models::DeleteResponse delete_variant_link(id)
Delete a variant procurement link

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | Link ID | [required] |

### Return type

[**models::DeleteResponse**](DeleteResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## upsert_variant_link

> models::VariantLinkResponse upsert_variant_link(upsert_variant_link_request)
Upsert a variant procurement link

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**upsert_variant_link_request** | [**UpsertVariantLinkRequest**](UpsertVariantLinkRequest.md) |  | [required] |

### Return type

[**models::VariantLinkResponse**](VariantLinkResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

