# MergeRequestFullState

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**sequence** | **i32** |  | 
**target_branch** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**source_branch** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**source_repo_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**target_repo_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**title** | **String** |  | 
**merge_status** | **i32** |  | 
**description** | Option<**String**> |  | [optional]
**author_id** | [**uuid::Uuid**](uuid::Uuid.md) |  | 
**changes** | [**Vec<models::ChangePair>**](ChangePair.md) |  | 
**created_at** | **i64** |  | 
**updated_at** | **i64** |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


