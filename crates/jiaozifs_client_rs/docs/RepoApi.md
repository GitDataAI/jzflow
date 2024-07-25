# \RepoApi

All URIs are relative to *http://localhost:34913/api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**change_visible**](RepoApi.md#change_visible) | **POST** /repos/{owner}/{repository}/visible | change repository visible(true for public, false for private)
[**create_repository**](RepoApi.md#create_repository) | **POST** /users/repos | create repository
[**delete_repository**](RepoApi.md#delete_repository) | **DELETE** /repos/{owner}/{repository} | delete repository
[**get_commits_in_ref**](RepoApi.md#get_commits_in_ref) | **GET** /repos/{owner}/{repository}/commits | get commits in ref
[**get_repository**](RepoApi.md#get_repository) | **GET** /repos/{owner}/{repository} | get repository
[**list_public_repository**](RepoApi.md#list_public_repository) | **GET** /repos/public | list public repository in all system
[**list_repository**](RepoApi.md#list_repository) | **GET** /users/{owner}/repos | list repository in specific owner
[**list_repository_of_authenticated_user_double_quote**](RepoApi.md#list_repository_of_authenticated_user_double_quote) | **GET** /users/repos | list repository
[**update_repository**](RepoApi.md#update_repository) | **POST** /repos/{owner}/{repository} | update repository



## change_visible

> change_visible(owner, repository, visible)
change repository visible(true for public, false for private)

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**owner** | **String** |  | [required] |
**repository** | **String** |  | [required] |
**visible** | **bool** |  | [required] |

### Return type

 (empty response body)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## create_repository

> models::Repository create_repository(create_repository)
create repository

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**create_repository** | [**CreateRepository**](CreateRepository.md) |  | [required] |

### Return type

[**models::Repository**](Repository.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## delete_repository

> delete_repository(owner, repository, is_clean_data)
delete repository

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**owner** | **String** |  | [required] |
**repository** | **String** |  | [required] |
**is_clean_data** | Option<**bool**> |  |  |

### Return type

 (empty response body)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_commits_in_ref

> Vec<models::Commit> get_commits_in_ref(owner, repository, after, amount, ref_name)
get commits in ref

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**owner** | **String** |  | [required] |
**repository** | **String** |  | [required] |
**after** | Option<**i64**> | return items after this value |  |
**amount** | Option<**i32**> | how many items to return |  |[default to 100]
**ref_name** | Option<**String**> | ref(branch/tag) name |  |

### Return type

[**Vec<models::Commit>**](Commit.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_repository

> models::Repository get_repository(owner, repository)
get repository

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**owner** | **String** |  | [required] |
**repository** | **String** |  | [required] |

### Return type

[**models::Repository**](Repository.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_public_repository

> models::RepositoryList list_public_repository(prefix, after, amount)
list public repository in all system

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**prefix** | Option<**String**> | return items prefixed with this value |  |
**after** | Option<**i64**> | return items after this value |  |
**amount** | Option<**i32**> | how many items to return |  |[default to 100]

### Return type

[**models::RepositoryList**](RepositoryList.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_repository

> models::RepositoryList list_repository(owner, prefix, after, amount)
list repository in specific owner

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**owner** | **String** |  | [required] |
**prefix** | Option<**String**> | return items prefixed with this value |  |
**after** | Option<**i64**> | return items after this value |  |
**amount** | Option<**i32**> | how many items to return |  |[default to 100]

### Return type

[**models::RepositoryList**](RepositoryList.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_repository_of_authenticated_user_double_quote

> models::RepositoryList list_repository_of_authenticated_user_double_quote(prefix, after, amount)
list repository

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**prefix** | Option<**String**> | return items prefixed with this value |  |
**after** | Option<**i64**> | return items after this value |  |
**amount** | Option<**i32**> | how many items to return |  |[default to 100]

### Return type

[**models::RepositoryList**](RepositoryList.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## update_repository

> update_repository(owner, repository, update_repository)
update repository

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**owner** | **String** |  | [required] |
**repository** | **String** |  | [required] |
**update_repository** | [**UpdateRepository**](UpdateRepository.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

