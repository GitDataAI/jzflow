# \WipApi

All URIs are relative to *http://localhost:34913/api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**commit_wip**](WipApi.md#commit_wip) | **POST** /wip/{owner}/{repository}/commit | commit working in process to branch
[**delete_wip**](WipApi.md#delete_wip) | **DELETE** /wip/{owner}/{repository} | remove working in process
[**get_wip**](WipApi.md#get_wip) | **GET** /wip/{owner}/{repository} | get working in process
[**get_wip_changes**](WipApi.md#get_wip_changes) | **GET** /wip/{owner}/{repository}/changes | get working in process changes
[**list_wip**](WipApi.md#list_wip) | **GET** /wip/{owner}/{repository}/list | list wip in specific project and user
[**revert_wip_changes**](WipApi.md#revert_wip_changes) | **POST** /wip/{owner}/{repository}/revert | revert changes in working in process, empty path will revert all
[**update_wip**](WipApi.md#update_wip) | **POST** /wip/{owner}/{repository} | update wip



## commit_wip

> models::Wip commit_wip(owner, repository, ref_name, msg)
commit working in process to branch

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**owner** | **String** |  | [required] |
**repository** | **String** |  | [required] |
**ref_name** | **String** | ref name | [required] |
**msg** | **String** | commit message | [required] |

### Return type

[**models::Wip**](Wip.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## delete_wip

> delete_wip(repository, owner, ref_name)
remove working in process

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**repository** | **String** |  | [required] |
**owner** | **String** |  | [required] |
**ref_name** | **String** | ref name | [required] |

### Return type

 (empty response body)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_wip

> models::Wip get_wip(repository, owner, ref_name)
get working in process

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**repository** | **String** |  | [required] |
**owner** | **String** |  | [required] |
**ref_name** | **String** | ref name | [required] |

### Return type

[**models::Wip**](Wip.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_wip_changes

> Vec<models::Change> get_wip_changes(owner, repository, ref_name, path)
get working in process changes

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**owner** | **String** |  | [required] |
**repository** | **String** |  | [required] |
**ref_name** | **String** | ref name | [required] |
**path** | Option<**String**> | path |  |

### Return type

[**Vec<models::Change>**](Change.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_wip

> Vec<models::Wip> list_wip(owner, repository)
list wip in specific project and user

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**owner** | **String** |  | [required] |
**repository** | **String** |  | [required] |

### Return type

[**Vec<models::Wip>**](Wip.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## revert_wip_changes

> revert_wip_changes(repository, owner, ref_name, path_prefix)
revert changes in working in process, empty path will revert all

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**repository** | **String** |  | [required] |
**owner** | **String** |  | [required] |
**ref_name** | **String** | ref name | [required] |
**path_prefix** | Option<**String**> | prefix of path |  |

### Return type

 (empty response body)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## update_wip

> update_wip(repository, owner, ref_name, update_wip)
update wip

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**repository** | **String** |  | [required] |
**owner** | **String** |  | [required] |
**ref_name** | **String** | ref name | [required] |
**update_wip** | [**UpdateWip**](UpdateWip.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

