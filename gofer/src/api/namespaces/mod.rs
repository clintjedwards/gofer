use crate::api::{epoch, validate, Api};
use crate::storage;
use gofer_proto::{
    CreateNamespaceRequest, CreateNamespaceResponse, DeleteNamespaceRequest,
    DeleteNamespaceResponse, GetNamespaceRequest, GetNamespaceResponse, ListNamespacesRequest,
    ListNamespacesResponse, Namespace, UpdateNamespaceRequest, UpdateNamespaceResponse,
};
use tonic::{Response, Status};

impl Api {
    pub async fn list_namespaces_handler(
        &self,
        args: ListNamespacesRequest,
    ) -> Result<Response<ListNamespacesResponse>, Status> {
        self.storage
            .list_namespaces(args.offset, args.limit)
            .await
            .map(|namespaces| {
                Response::new(ListNamespacesResponse {
                    namespaces: namespaces.into_iter().map(Namespace::from).collect(),
                })
            })
            .map_err(|e| Status::internal(e.to_string()))
    }

    pub async fn create_namespace_handler(
        &self,
        args: CreateNamespaceRequest,
    ) -> Result<Response<CreateNamespaceResponse>, Status> {
        validate::arg("id", args.id.clone(), vec![validate::is_valid_identifier])?;
        validate::arg("name", args.name.clone(), vec![validate::not_empty_str])?;

        let new_namespace =
            gofer_models::namespace::Namespace::new(&args.id, &args.name, &args.description);

        self.storage
            .create_namespace(&new_namespace)
            .await
            .map_err(|e| match e {
                storage::StorageError::Exists => Status::already_exists(format!(
                    "namespace with id '{}' already exists",
                    new_namespace.id
                )),
                _ => Status::internal(e.to_string()),
            })?;

        self.event_bus
            .publish(gofer_models::event::Kind::CreatedNamespace {
                namespace_id: new_namespace.id.clone(),
            })
            .await;
        Ok(Response::new(CreateNamespaceResponse {
            namespace: Some(new_namespace.into()),
        }))
    }

    pub async fn get_namespace_handler(
        &self,
        args: GetNamespaceRequest,
    ) -> Result<Response<GetNamespaceResponse>, Status> {
        validate::arg("id", args.id.clone(), vec![validate::is_valid_identifier])?;

        self.storage
            .get_namespace(&args.id)
            .await
            .map(|namespace| {
                Response::new(GetNamespaceResponse {
                    namespace: Some(namespace.into()),
                })
            })
            .map_err(|e| match e {
                storage::StorageError::NotFound => {
                    Status::not_found(format!("namespace with id '{}' does not exist", &args.id))
                }
                _ => Status::internal(e.to_string()),
            })
    }

    pub async fn update_namespace_handler(
        &self,
        args: UpdateNamespaceRequest,
    ) -> Result<Response<UpdateNamespaceResponse>, Status> {
        validate::arg("id", args.id.clone(), vec![validate::is_valid_identifier])?;
        validate::arg("name", args.name.clone(), vec![validate::not_empty_str])?;

        self.storage
            .update_namespace(&gofer_models::namespace::Namespace {
                id: args.id.clone(),
                name: args.name.clone(),
                description: args.description.clone(),
                created: 0,
                modified: epoch(),
            })
            .await
            .map_err(|e| match e {
                storage::StorageError::NotFound => {
                    Status::not_found(format!("namespace with id '{}' does not exist", &args.id))
                }
                _ => Status::internal(e.to_string()),
            })?;

        Ok(Response::new(UpdateNamespaceResponse {}))
    }

    pub async fn delete_namespace_handler(
        &self,
        args: DeleteNamespaceRequest,
    ) -> Result<Response<DeleteNamespaceResponse>, Status> {
        validate::arg("id", args.id.clone(), vec![validate::is_valid_identifier])?;

        self.storage
            .delete_namespace(&args.id)
            .await
            .map_err(|e| match e {
                storage::StorageError::NotFound => {
                    Status::not_found(format!("namespace with id '{}' does not exist", &args.id))
                }
                _ => Status::internal(e.to_string()),
            })?;

        self.event_bus
            .publish(gofer_models::event::Kind::DeletedNamespace {
                namespace_id: args.id.clone(),
            })
            .await;

        Ok(Response::new(DeleteNamespaceResponse {}))
    }
}
