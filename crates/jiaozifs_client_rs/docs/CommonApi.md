# \CommonApi

All URIs are relative to *http://localhost:34913/api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_setup_state**](CommonApi.md#get_setup_state) | **GET** /setup | check if jiaozifs setup
[**get_version**](CommonApi.md#get_version) | **GET** /version | return program and runtime version



## get_setup_state

> models::SetupState get_setup_state()
check if jiaozifs setup

### Parameters

This endpoint does not need any parameter.

### Return type

[**models::SetupState**](SetupState.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_version

> models::VersionResult get_version()
return program and runtime version

### Parameters

This endpoint does not need any parameter.

### Return type

[**models::VersionResult**](VersionResult.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

