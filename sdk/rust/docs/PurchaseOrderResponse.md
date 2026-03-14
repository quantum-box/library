# PurchaseOrderResponse

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**billing_info** | Option<**String**> | Billing information ID | [optional]
**client_id** | **String** | Client ID | 
**currency** | **String** | Transaction currency | 
**delivery_date** | Option<**String**> | Delivery date | [optional]
**id** | **String** | Purchase order ID | 
**invoice_address** | Option<**String**> | Invoice address ID | [optional]
**line_items** | [**Vec<models::LineItemResponse>**](LineItemResponse.md) | Line items | 
**order_date** | **String** | Order date | 
**quotes_id** | **String** | Quote ID | 
**software_tenant_id** | Option<**String**> | Software tenant ID | [optional]
**status** | **String** | Order status | 
**subtotal** | **f32** | Subtotal | 
**tax** | **f32** | Tax rate | 
**tenant_id** | **String** | Tenant ID | 
**total** | **f32** | Total amount | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


