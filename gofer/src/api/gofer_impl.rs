use futures::Stream;
use gofer_proto::{gofer_server::Gofer, *};
use std::{ops::Deref, pin::Pin};
use tonic::{Request, Response, Status};

use super::ApiWrapper;

// Since we can't implement this trait over many files each function here just calls out to a clone function
// located in other, more neatly organized files.
#[tonic::async_trait]
impl Gofer for ApiWrapper {
    async fn get_system_info(
        &self,
        _: Request<GetSystemInfoRequest>,
    ) -> Result<Response<GetSystemInfoResponse>, Status> {
        self.get_system_info_handler()
    }

    async fn list_namespaces(
        &self,
        request: Request<ListNamespacesRequest>,
    ) -> Result<Response<ListNamespacesResponse>, Status> {
        let args = request.into_inner();
        self.list_namespaces_handler(args).await
    }

    async fn create_namespace(
        &self,
        request: Request<CreateNamespaceRequest>,
    ) -> Result<Response<CreateNamespaceResponse>, Status> {
        let args = request.into_inner();
        self.deref().clone().create_namespace_handler(args).await
    }

    async fn get_namespace(
        &self,
        request: Request<GetNamespaceRequest>,
    ) -> Result<Response<GetNamespaceResponse>, Status> {
        let args = request.into_inner();
        self.get_namespace_handler(args).await
    }

    async fn update_namespace(
        &self,
        request: Request<UpdateNamespaceRequest>,
    ) -> Result<Response<UpdateNamespaceResponse>, Status> {
        let args = request.into_inner();
        self.update_namespace_handler(args).await
    }

    async fn delete_namespace(
        &self,
        request: Request<DeleteNamespaceRequest>,
    ) -> Result<Response<DeleteNamespaceResponse>, Status> {
        let args = request.into_inner();
        self.deref().clone().delete_namespace_handler(args).await
    }

    async fn list_pipelines(
        &self,
        request: Request<ListPipelinesRequest>,
    ) -> Result<Response<ListPipelinesResponse>, Status> {
        let args = request.into_inner();
        self.list_pipelines_handler(args).await
    }

    async fn create_pipeline(
        &self,
        request: Request<CreatePipelineRequest>,
    ) -> Result<Response<CreatePipelineResponse>, Status> {
        let args = request.into_inner();
        self.deref().clone().create_pipeline_handler(args).await
    }

    async fn get_pipeline(
        &self,
        request: Request<GetPipelineRequest>,
    ) -> Result<Response<GetPipelineResponse>, Status> {
        let args = request.into_inner();
        self.get_pipeline_handler(args).await
    }

    async fn enable_pipeline(
        &self,
        request: Request<EnablePipelineRequest>,
    ) -> Result<Response<EnablePipelineResponse>, Status> {
        let args = request.into_inner();
        self.deref().clone().enable_pipeline_handler(args).await
    }

    async fn disable_pipeline(
        &self,
        request: Request<DisablePipelineRequest>,
    ) -> Result<Response<DisablePipelineResponse>, Status> {
        let args = request.into_inner();
        self.deref().clone().disable_pipeline_handler(args).await
    }

    async fn update_pipeline(
        &self,
        request: Request<UpdatePipelineRequest>,
    ) -> Result<Response<UpdatePipelineResponse>, Status> {
        let args = request.into_inner();
        self.update_pipeline_handler(args).await
    }

    async fn delete_pipeline(
        &self,
        request: Request<DeletePipelineRequest>,
    ) -> Result<Response<DeletePipelineResponse>, Status> {
        let args = request.into_inner();
        self.deref().clone().delete_pipeline_handler(args).await
    }

    async fn get_run(
        &self,
        request: Request<GetRunRequest>,
    ) -> Result<Response<GetRunResponse>, Status> {
        let args = request.into_inner();
        self.get_run_handler(args).await
    }

    async fn list_runs(
        &self,
        request: Request<ListRunsRequest>,
    ) -> Result<Response<ListRunsResponse>, Status> {
        let args = request.into_inner();
        self.list_runs_handler(args).await
    }

    async fn start_run(
        &self,
        request: Request<StartRunRequest>,
    ) -> Result<Response<StartRunResponse>, Status> {
        let args = request.into_inner();
        self.deref().clone().start_run_handler(args).await
    }

    async fn retry_run(
        &self,
        request: Request<RetryRunRequest>,
    ) -> Result<Response<RetryRunResponse>, Status> {
        let args = request.into_inner();
        self.deref().clone().retry_run_handler(args).await
    }

    async fn cancel_run(
        &self,
        request: Request<CancelRunRequest>,
    ) -> Result<Response<CancelRunResponse>, Status> {
        let args = request.into_inner();
        self.deref().clone().cancel_run_handler(args).await
    }

    async fn cancel_all_runs(
        &self,
        request: Request<CancelAllRunsRequest>,
    ) -> Result<Response<CancelAllRunsResponse>, Status> {
        let args = request.into_inner();
        self.deref().clone().cancel_all_runs_handler(args).await
    }

    async fn get_task_run(
        &self,
        request: Request<GetTaskRunRequest>,
    ) -> Result<Response<GetTaskRunResponse>, Status> {
        let args = request.into_inner();
        self.get_task_run_handler(args).await
    }

    async fn list_task_runs(
        &self,
        request: Request<ListTaskRunsRequest>,
    ) -> Result<Response<ListTaskRunsResponse>, Status> {
        let args = request.into_inner();
        self.list_task_runs_handler(args).await
    }

    async fn cancel_task_run(
        &self,
        request: Request<CancelTaskRunRequest>,
    ) -> Result<Response<CancelTaskRunResponse>, Status> {
        let args = request.into_inner();
        self.cancel_task_run_handler(args).await
    }

    type GetTaskRunLogsStream =
        Pin<Box<dyn Stream<Item = Result<GetTaskRunLogsResponse, Status>> + Send>>;

    async fn get_task_run_logs(
        &self,
        request: Request<GetTaskRunLogsRequest>,
    ) -> Result<Response<Self::GetTaskRunLogsStream>, Status> {
        let args = request.into_inner();
        self.deref().clone().get_task_run_logs_handler(args).await
    }

    async fn delete_task_run_logs(
        &self,
        request: Request<DeleteTaskRunLogsRequest>,
    ) -> Result<Response<DeleteTaskRunLogsResponse>, Status> {
        let args = request.into_inner();
        self.delete_task_run_logs_handler(args).await
    }

    async fn get_trigger(
        &self,
        request: Request<GetTriggerRequest>,
    ) -> Result<Response<GetTriggerResponse>, Status> {
        let args = request.into_inner();
        self.get_trigger_handler(args).await
    }

    async fn list_triggers(
        &self,
        request: Request<ListTriggersRequest>,
    ) -> Result<Response<ListTriggersResponse>, Status> {
        let args = request.into_inner();
        self.list_triggers_handler(args).await
    }

    async fn install_trigger(
        &self,
        request: Request<InstallTriggerRequest>,
    ) -> Result<Response<InstallTriggerResponse>, Status> {
        let args = request.into_inner();
        self.install_trigger_handler(args).await
    }

    async fn uninstall_trigger(
        &self,
        request: Request<UninstallTriggerRequest>,
    ) -> Result<Response<UninstallTriggerResponse>, Status> {
        let args = request.into_inner();
        self.uninstall_trigger_handler(args).await
    }

    async fn enable_trigger(
        &self,
        request: Request<EnableTriggerRequest>,
    ) -> Result<Response<EnableTriggerResponse>, Status> {
        let args = request.into_inner();
        self.enable_trigger_handler(args).await
    }

    async fn disable_trigger(
        &self,
        request: Request<DisableTriggerRequest>,
    ) -> Result<Response<DisableTriggerResponse>, Status> {
        let args = request.into_inner();
        self.disable_trigger_handler(args).await
    }

    async fn get_trigger_install_instructions(
        &self,
        request: Request<GetTriggerInstallInstructionsRequest>,
    ) -> Result<Response<GetTriggerInstallInstructionsResponse>, Status> {
        let args = request.into_inner();
        self.get_trigger_install_instructions_handler(args).await
    }

    async fn get_common_task(
        &self,
        request: Request<GetCommonTaskRequest>,
    ) -> Result<Response<GetCommonTaskResponse>, Status> {
        todo!()
    }

    async fn list_common_tasks(
        &self,
        request: Request<ListCommonTasksRequest>,
    ) -> Result<Response<ListCommonTasksResponse>, Status> {
        todo!()
    }

    async fn install_common_task(
        &self,
        request: Request<InstallCommonTaskRequest>,
    ) -> Result<Response<InstallCommonTaskResponse>, Status> {
        todo!()
    }

    async fn uninstall_common_task(
        &self,
        request: Request<UninstallCommonTaskRequest>,
    ) -> Result<Response<UninstallCommonTaskResponse>, Status> {
        todo!()
    }

    async fn enable_common_task(
        &self,
        request: Request<EnableCommonTaskRequest>,
    ) -> Result<Response<EnableCommonTaskResponse>, Status> {
        todo!()
    }

    async fn disable_common_task(
        &self,
        request: Request<DisableCommonTaskRequest>,
    ) -> Result<Response<DisableCommonTaskResponse>, Status> {
        todo!()
    }

    async fn get_event(
        &self,
        request: Request<GetEventRequest>,
    ) -> Result<Response<GetEventResponse>, Status> {
        let args = request.into_inner();
        self.get_event_handler(args).await
    }

    type ListEventsStream = Pin<Box<dyn Stream<Item = Result<ListEventsResponse, Status>> + Send>>;

    async fn list_events(
        &self,
        request: Request<ListEventsRequest>,
    ) -> Result<Response<Self::ListEventsStream>, Status> {
        let args = request.into_inner();
        self.deref().clone().list_events_handler(args).await
    }
}
