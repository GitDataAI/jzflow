# \AksksApi

All URIs are relative to *http://localhost:34913/api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_aksk**](AksksApi.md#create_aksk) | **POST** /users/aksk | create aksk
[**delete_aksk**](AksksApi.md#delete_aksk) | **DELETE** /users/aksk | delete aksk
[**get_aksk**](AksksApi.md#get_aksk) | **GET** /users/aksk | get aksk
[**list_aksks**](AksksApi.md#list_aksks) | **GET** /users/aksks | list aksks



## create_aksk

> models::Aksk create_aksk(description)
create aksk

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**description** | Option<**String**> |  |  |

### Return type

[**models::Aksk**](Aksk.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## delete_aksk

> delete_aksk(id, access_key)
delete aksk

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | Option<**uuid::Uuid**> |  |  |
**access_key** | Option<**String**> |  |  |

### Return type

 (empty response body)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_aksk

> models::SafeAksk get_aksk(id, access_key)
get aksk

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | Option<**uuid::Uuid**> |  |  |
**access_key** | Option<**String**> |  |  |

### Return type

[**models::SafeAksk**](SafeAksk.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_aksks

> models::AkskList list_aksks(after, amount)
list aksks

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**after** | Option<**i64**> | return items after this value |  |
**amount** | Option<**i32**> | how many items to return |  |[default to 100]

### Return type

[**models::AkskList**](AkskList.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

