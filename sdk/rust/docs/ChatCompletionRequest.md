# ChatCompletionRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**communication_style** | Option<**String**> | Communication style preference | [optional]
**explanation_style** | Option<**String**> | Explanation style preference | [optional]
**frequency_penalty** | Option<**f32**> | Number between -2.0 and 2.0 | [optional][default to 0]
**max_completion_tokens** | Option<**i32**> | The maximum number of tokens to generate | [optional][default to 10000]
**memory_settings** | Option<[**models::MemorySettingsRequest**](MemorySettingsRequest.md)> | Memory settings for context building | [optional]
**messages** | [**Vec<models::Message>**](Message.md) | A list of messages comprising the conversation so far | 
**model** | Option<**String**> | ID of the model to use | [optional]
**n** | Option<**i32**> | How many chat completion choices to generate | [optional][default to 1]
**presence_penalty** | Option<**f32**> | Number between -2.0 and 2.0 | [optional][default to 0]
**response_format** | Option<[**models::ResponseFormat**](ResponseFormat.md)> | Format to return the response in | [optional]
**stream** | Option<**bool**> | Whether to stream back partial progress | [optional][default to false]
**technical_level** | Option<**String**> | Technical level preference | [optional]
**temperature** | Option<**f32**> | What sampling temperature to use, between 0 and 2 | [optional][default to 1.0]
**tool_choice** | Option<[**models::ToolChoice**](ToolChoice.md)> | Controls which (if any) function is called by the model | [optional]
**tools** | Option<[**Vec<models::Tool>**](Tool.md)> | A list of tools the model may call | [optional]
**top_p** | Option<**f32**> | An alternative to sampling with temperature | [optional][default to 1.0]
**user** | Option<**String**> | A unique identifier representing your end-user | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


