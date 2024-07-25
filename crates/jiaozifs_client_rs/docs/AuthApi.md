# \AuthApi

All URIs are relative to *http://localhost:34913/api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_user_info**](AuthApi.md#get_user_info) | **GET** /users/user | get information of the currently logged-in user
[**login**](AuthApi.md#login) | **POST** /auth/login | perform a login
[**logout**](AuthApi.md#logout) | **POST** /auth/logout | perform a logout
[**refresh_token**](AuthApi.md#refresh_token) | **GET** /users/refreshtoken | refresh token for more time
[**register**](AuthApi.md#register) | **POST** /users/register | perform user registration



## get_user_info

> models::UserInfo get_user_info()
get information of the currently logged-in user

### Parameters

This endpoint does not need any parameter.

### Return type

[**models::UserInfo**](UserInfo.md)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## login

> models::AuthenticationToken login(login_request)
perform a login

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**login_request** | Option<[**LoginRequest**](LoginRequest.md)> |  |  |

### Return type

[**models::AuthenticationToken**](AuthenticationToken.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## logout

> logout()
perform a logout

### Parameters

This endpoint does not need any parameter.

### Return type

 (empty response body)

### Authorization

[basic_auth](../README.md#basic_auth), [cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## refresh_token

> models::AuthenticationToken refresh_token()
refresh token for more time

### Parameters

This endpoint does not need any parameter.

### Return type

[**models::AuthenticationToken**](AuthenticationToken.md)

### Authorization

[cookie_auth](../README.md#cookie_auth), [jwt_token](../README.md#jwt_token)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## register

> models::UserInfo register(user_register_info)
perform user registration

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_register_info** | Option<[**UserRegisterInfo**](UserRegisterInfo.md)> |  |  |

### Return type

[**models::UserInfo**](UserInfo.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

