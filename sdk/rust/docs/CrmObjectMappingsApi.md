# \CrmObjectMappingsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_object_mapping**](CrmObjectMappingsApi.md#create_object_mapping) | **POST** /v1/crm/object-mappings | Create an object mapping
[**get_object_mappings**](CrmObjectMappingsApi.md#get_object_mappings) | **GET** /v1/crm/object-mappings | Get object mappings by entity ID and object name



## create_object_mapping

> models::ObjectMappingResponse create_object_mapping(create_object_mapping_request)
Create an object mapping

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**create_object_mapping_request** | [**CreateObjectMappingRequest**](CreateObjectMappingRequest.md) |  | [required] |

### Return type

[**models::ObjectMappingResponse**](ObjectMappingResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_object_mappings

> models::ObjectMappingListResponse get_object_mappings(entity_id, object_name)
Get object mappings by entity ID and object name

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**entity_id** | **String** | Entity ID to look up | [required] |
**object_name** | **String** | Object type (Deal, Product, etc.) | [required] |

### Return type

[**models::ObjectMappingListResponse**](ObjectMappingListResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

