# \CommitApi

All URIs are relative to *http://localhost:34913/api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**compare_commit**](CommitApi.md#compare_commit) | **GET** /repos/{owner}/{repository}/compare/{basehead} | compare two commit
[**get_commit_changes**](CommitApi.md#get_commit_changes) | **GET** /repos/{owner}/{repository}/changes/{commit_id} | get changes in commit
[**get_entries_in_ref**](CommitApi.md#get_entries_in_ref) | **GET** /repos/{owner}/{repository}/contents | list entries in ref



## compare_commit

> Vec<models::Change> compare_commit(owner, repository, basehead, path)
compare two commit

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**owner** | **String** |  | [required] |
**repository** | **String** |  | [required] |
**basehead** | **String** |  | [required] |
**path** | Option<**String**> | specific path, if not specific return entries in root |  |

### Return type

[**Vec<models::Change>**](Change.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_commit_changes

> Vec<models::Change> get_commit_changes(owner, repository, commit_id, path)
get changes in commit

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**owner** | **String** |  | [required] |
**repository** | **String** |  | [required] |
**commit_id** | **String** |  | [required] |
**path** | Option<**String**> | specific path, if not specific return entries in root |  |

### Return type

[**Vec<models::Change>**](Change.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_entries_in_ref

> Vec<models::FullTreeEntry> get_entries_in_ref(owner, repository, r#type, path, r#ref)
list entries in ref

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**owner** | **String** |  | [required] |
**repository** | **String** |  | [required] |
**r#type** | [**RefType**](.md) | type indicate to retrieve from wip/branch/tag/commit, default branch | [required] |
**path** | Option<**String**> | specific path, if not specific return entries in root |  |
**r#ref** | Option<**String**> | specific( ref name, tag name, commit hash), for wip and branchm, branch name default to repository default branch(HEAD), |  |

### Return type

[**Vec<models::FullTreeEntry>**](FullTreeEntry.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

