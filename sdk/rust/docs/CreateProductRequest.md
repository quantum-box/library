# CreateProductRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**billing_cycle** | **String** | Billing cycle: MONTHLY, YEARLY, ONE_TIME | 
**create_crm** | Option<**bool**> | Whether to sync with CRM | [optional]
**description** | Option<**String**> | Product description | [optional]
**id** | Option<**String**> | Optional product ID (auto-generated if omitted) | [optional]
**image_file_ids** | Option<**Vec<String>**> | File IDs for product images | [optional]
**jan_code** | Option<**String**> | JAN code | [optional]
**kind** | **String** | Product type: PLAN or PRODUCT | 
**list_price** | **i32** | List price in smallest currency unit | 
**name** | **String** | Product name | 
**publication_description** | Option<**String**> | Publication display description | [optional]
**publication_name** | Option<**String**> | Publication display name | [optional]
**publication_status** | Option<**String**> | Publication status | [optional]
**sku_code** | Option<**String**> | SKU code | [optional]
**status** | **String** | Product status: DRAFT, ACTIVE, ARCHIVED | 
**upc_code** | Option<**String**> | UPC code | [optional]
**variations** | Option<[**Vec<models::ProductVariationRequest>**](ProductVariationRequest.md)> | Product variations | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


