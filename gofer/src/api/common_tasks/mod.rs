use crate::{
    api::{epoch, validate, Api},
    scheduler, storage,
};
use futures::stream::StreamExt;
use gofer_models::{common_task, event};
use gofer_proto::{
    DisableCommonTaskRequest, DisableCommonTaskResponse, EnableCommonTaskRequest,
    EnableCommonTaskResponse, GetCommonTaskInstallInstructionsRequest,
    GetCommonTaskInstallInstructionsResponse, GetCommonTaskRequest, GetCommonTaskResponse,
    InstallCommonTaskRequest, InstallCommonTaskResponse, ListCommonTasksRequest,
    ListCommonTasksResponse, UninstallCommonTaskRequest, UninstallCommonTaskResponse,
};
use nanoid::nanoid;
use slog_scope::info;
use std::collections::HashMap;
use tonic::{Response, Status};

impl Api {
    pub async fn install_common_task_handler(
        &self,
        args: InstallCommonTaskRequest,
    ) -> Result<Response<InstallCommonTaskResponse>, Status> {
        todo!()
    }

    pub async fn get_common_task_install_instructions_handler(
        &self,
        args: GetCommonTaskInstallInstructionsRequest,
    ) -> Result<Response<GetCommonTaskInstallInstructionsResponse>, Status> {
        todo!()
    }

    pub async fn get_common_task_handler(
        &self,
        args: GetCommonTaskRequest,
    ) -> Result<Response<GetCommonTaskResponse>, Status> {
        todo!()
    }

    pub async fn list_common_tasks_handler(
        &self,
        _: ListCommonTasksRequest,
    ) -> Result<Response<ListCommonTasksResponse>, Status> {
        todo!()
    }

    pub async fn uninstall_common_task_handler(
        &self,
        args: UninstallCommonTaskRequest,
    ) -> Result<Response<UninstallCommonTaskResponse>, Status> {
        todo!()
    }

    pub async fn enable_common_task_handler(
        &self,
        args: EnableCommonTaskRequest,
    ) -> Result<Response<EnableCommonTaskResponse>, Status> {
        todo!()
    }

    pub async fn disable_common_task_handler(
        &self,
        args: DisableCommonTaskRequest,
    ) -> Result<Response<DisableCommonTaskResponse>, Status> {
        todo!()
    }
}
