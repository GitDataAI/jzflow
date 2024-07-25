# \MergerequestApi

All URIs are relative to *http://localhost:34913/api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_merge_request**](MergerequestApi.md#create_merge_request) | **POST** /repos/{owner}/{repository}/mergerequest | create merge request
[**get_merge_request**](MergerequestApi.md#get_merge_request) | **GET** /repos/{owner}/{repository}/mergerequest/{mrSeq} | get merge request
[**list_merge_requests**](MergerequestApi.md#list_merge_requests) | **GET** /repos/{owner}/{repository}/mergerequest | get list of merge request in repository
[**merge**](MergerequestApi.md#merge) | **POST** /repos/{owner}/{repository}/mergerequest/{mrSeq}/merge | merge a mergerequest
[**update_merge_request**](MergerequestApi.md#update_merge_request) | **POST** /repos/{owner}/{repository}/mergerequest/{mrSeq} | update merge request



## create_merge_request

> models::MergeRequest create_merge_request(owner, repository, create_merge_request)
create merge request

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**owner** | **String** |  | [required] |
**repository** | **String** |  | [required] |
**create_merge_request** | [**CreateMergeRequest**](CreateMergeRequest.md) |  | [required] |

### Return type

[**models::MergeRequest**](MergeRequest.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_merge_request

> models::MergeRequestFullState get_merge_request(owner, repository, mr_seq)
get merge request

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**owner** | **String** |  | [required] |
**repository** | **String** |  | [required] |
**mr_seq** | **i32** |  | [required] |

### Return type

[**models::MergeRequestFullState**](MergeRequestFullState.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_merge_requests

> models::MergeRequestList list_merge_requests(owner, repository, after, amount, state)
get list of merge request in repository

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**owner** | **String** |  | [required] |
**repository** | **String** |  | [required] |
**after** | Option<**i64**> | return items after this value |  |
**amount** | Option<**i32**> | how many items to return |  |[default to 100]
**state** | Option<**i32**> |  |  |

### Return type

[**models::MergeRequestList**](MergeRequestList.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## merge

> Vec<models::Commit> merge(owner, repository, mr_seq, merge_merge_request)
merge a mergerequest

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**owner** | **String** |  | [required] |
**repository** | **String** |  | [required] |
**mr_seq** | **i32** |  | [required] |
**merge_merge_request** | [**MergeMergeRequest**](MergeMergeRequest.md) |  | [required] |

### Return type

[**Vec<models::Commit>**](Commit.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## update_merge_request

> update_merge_request(owner, repository, mr_seq, update_merge_request)
update merge request

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**owner** | **String** |  | [required] |
**repository** | **String** |  | [required] |
**mr_seq** | **i32** |  | [required] |
**update_merge_request** | [**UpdateMergeRequest**](UpdateMergeRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

