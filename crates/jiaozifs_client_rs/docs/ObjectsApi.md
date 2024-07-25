# \ObjectsApi

All URIs are relative to *http://localhost:34913/api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**delete_object**](ObjectsApi.md#delete_object) | **DELETE** /object/{owner}/{repository} | delete object. Missing objects will not return a NotFound error.
[**get_files**](ObjectsApi.md#get_files) | **GET** /object/{owner}/{repository}/files | get files by pattern
[**get_object**](ObjectsApi.md#get_object) | **GET** /object/{owner}/{repository} | get object content
[**head_object**](ObjectsApi.md#head_object) | **HEAD** /object/{owner}/{repository} | check if object exists
[**upload_object**](ObjectsApi.md#upload_object) | **POST** /object/{owner}/{repository} | 



## delete_object

> delete_object(owner, repository, ref_name, path)
delete object. Missing objects will not return a NotFound error.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**owner** | **String** |  | [required] |
**repository** | **String** |  | [required] |
**ref_name** | **String** | branch/tag to the ref | [required] |
**path** | **String** | relative to the ref | [required] |

### Return type

 (empty response body)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_files

> Vec<String> get_files(owner, repository, ref_name, r#type, pattern)
get files by pattern

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**owner** | **String** |  | [required] |
**repository** | **String** |  | [required] |
**ref_name** | **String** | branch/tag to the ref | [required] |
**r#type** | [**RefType**](.md) | files to retrieve from wip/branch/tag/commit, default branch | [required] |
**pattern** | Option<**String**> | glob pattern for match file path |  |

### Return type

**Vec<String>**

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_object

> std::path::PathBuf get_object(owner, repository, ref_name, path, r#type, range)
get object content

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**owner** | **String** |  | [required] |
**repository** | **String** |  | [required] |
**ref_name** | **String** | branch/tag to the ref | [required] |
**path** | **String** | relative to the ref | [required] |
**r#type** | [**RefType**](.md) | type indicate to retrieve from wip/branch/tag, default branch | [required] |
**range** | Option<**String**> | Byte range to retrieve |  |

### Return type

[**std::path::PathBuf**](std::path::PathBuf.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/octet-stream

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## head_object

> head_object(owner, repository, ref_name, path, r#type, range)
check if object exists

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**owner** | **String** |  | [required] |
**repository** | **String** |  | [required] |
**ref_name** | **String** | branch/tag to the ref | [required] |
**path** | **String** | relative to the ref | [required] |
**r#type** | [**RefType**](.md) | type indicate to retrieve from wip/branch/tag, default branch | [required] |
**range** | Option<**String**> | Byte range to retrieve |  |

### Return type

 (empty response body)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## upload_object

> models::ObjectStats upload_object(owner, repository, ref_name, path, is_replace, content)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**owner** | **String** |  | [required] |
**repository** | **String** |  | [required] |
**ref_name** | **String** | branch/tag to the ref | [required] |
**path** | **String** | relative to the ref | [required] |
**is_replace** | Option<**bool**> | indicate to replace existing object or not |  |
**content** | Option<**std::path::PathBuf**> | Only a single file per upload which must be named \\\"content\\\". |  |

### Return type

[**models::ObjectStats**](ObjectStats.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: multipart/form-data, application/octet-stream
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

