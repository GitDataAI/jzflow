# \ReposApi

All URIs are relative to *http://localhost:34913/api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_archive**](ReposApi.md#get_archive) | **GET** /repos/{owner}/{repository}/archive | get repo files archive



## get_archive

> std::path::PathBuf get_archive(owner, repository, archive_type, ref_type, ref_name)
get repo files archive

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**owner** | **String** |  | [required] |
**repository** | **String** |  | [required] |
**archive_type** | [**ArchiveType**](.md) | download zip or car files | [required] |
**ref_type** | [**RefType**](.md) | ref type only allow branch or tag | [required] |
**ref_name** | **String** | ref(branch/tag) name | [required] |

### Return type

[**std::path::PathBuf**](std::path::PathBuf.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/octet-stream

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

