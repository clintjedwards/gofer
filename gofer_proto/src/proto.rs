// Documentation for these models can be found in the "models" package as these
// are just protobuf representations of those models.
//
// Why represent what amounts to the same model twice in protobuf AND a struct
// you ask?
//
//   Essentially, because the separation of the transport layer and the
//   application layer is a good thing. There are probably many reasons, but
//   the most straightforward is that we might want to represent something in
//   the database or within the application that might not be easily
//   representable in protobuf, a structure mainly made for transport.
//
//   There might also be things that we don't want to expose outside and so
//   the separation gives us a chance to not mess that up by simply forgetting a
//   json:"-".

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Namespace {
    #[prost(string, tag="1")]
    pub id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub description: ::prost::alloc::string::String,
    #[prost(uint64, tag="4")]
    pub created: u64,
    #[prost(uint64, tag="5")]
    pub modified: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Variable {
    #[prost(string, tag="1")]
    pub key: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub value: ::prost::alloc::string::String,
    #[prost(enumeration="VariableOwner", tag="3")]
    pub owner: i32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Pipeline {
    #[prost(string, tag="1")]
    pub namespace: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub id: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag="4")]
    pub description: ::prost::alloc::string::String,
    #[prost(uint64, tag="5")]
    pub last_run_id: u64,
    #[prost(uint64, tag="6")]
    pub last_run_time: u64,
    #[prost(uint64, tag="7")]
    pub parallelism: u64,
    #[prost(uint64, tag="8")]
    pub created: u64,
    #[prost(uint64, tag="9")]
    pub modified: u64,
    #[prost(enumeration="pipeline::PipelineState", tag="10")]
    pub state: i32,
    #[prost(map="string, message", tag="11")]
    pub tasks: ::std::collections::HashMap<::prost::alloc::string::String, Task>,
    #[prost(map="string, message", tag="12")]
    pub triggers: ::std::collections::HashMap<::prost::alloc::string::String, PipelineTriggerSettings>,
    #[prost(map="string, message", tag="13")]
    pub notifiers: ::std::collections::HashMap<::prost::alloc::string::String, PipelineNotifierSettings>,
    #[prost(string, repeated, tag="14")]
    pub store_keys: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
/// Nested message and enum types in `Pipeline`.
pub mod pipeline {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum PipelineState {
        Unknown = 0,
        Active = 1,
        Disabled = 2,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PipelineConfig {
    #[prost(string, tag="1")]
    pub id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub description: ::prost::alloc::string::String,
    #[prost(uint64, tag="4")]
    pub parallelism: u64,
    #[prost(message, repeated, tag="5")]
    pub tasks: ::prost::alloc::vec::Vec<TaskConfig>,
    #[prost(message, repeated, tag="6")]
    pub triggers: ::prost::alloc::vec::Vec<PipelineTriggerConfig>,
    #[prost(message, repeated, tag="7")]
    pub notifiers: ::prost::alloc::vec::Vec<PipelineNotifierConfig>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Run {
    #[prost(string, tag="1")]
    pub namespace: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline: ::prost::alloc::string::String,
    #[prost(uint64, tag="3")]
    pub id: u64,
    #[prost(uint64, tag="4")]
    pub started: u64,
    #[prost(uint64, tag="5")]
    pub ended: u64,
    #[prost(enumeration="run::RunState", tag="6")]
    pub state: i32,
    #[prost(enumeration="run::RunStatus", tag="7")]
    pub status: i32,
    #[prost(message, optional, tag="8")]
    pub failure_info: ::core::option::Option<RunFailureInfo>,
    #[prost(string, repeated, tag="9")]
    pub task_runs: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(message, optional, tag="10")]
    pub trigger: ::core::option::Option<RunTriggerInfo>,
    #[prost(message, repeated, tag="11")]
    pub variables: ::prost::alloc::vec::Vec<Variable>,
    #[prost(message, optional, tag="12")]
    pub store_info: ::core::option::Option<RunStoreInfo>,
}
/// Nested message and enum types in `Run`.
pub mod run {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum RunState {
        Unknown = 0,
        Pending = 1,
        Running = 2,
        Complete = 3,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum RunStatus {
        Unknown = 0,
        Successful = 1,
        Failed = 2,
        Cancelled = 3,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RunFailureInfo {
    #[prost(enumeration="run_failure_info::RunFailureReason", tag="1")]
    pub reason: i32,
    #[prost(string, tag="2")]
    pub description: ::prost::alloc::string::String,
}
/// Nested message and enum types in `RunFailureInfo`.
pub mod run_failure_info {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum RunFailureReason {
        Unknown = 0,
        AbnormalExit = 1,
        SchedulerError = 2,
        FailedPrecondition = 3,
        UserCancelled = 4,
        AdminCancelled = 5,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RunTriggerInfo {
    #[prost(string, tag="1")]
    pub kind: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub label: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RunStoreInfo {
    #[prost(bool, tag="1")]
    pub is_expired: bool,
    #[prost(string, repeated, tag="2")]
    pub keys: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RegistryAuth {
    #[prost(string, tag="1")]
    pub user: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pass: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Exec {
    #[prost(string, tag="1")]
    pub shell: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub script: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Task {
    #[prost(string, tag="1")]
    pub id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub description: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub image: ::prost::alloc::string::String,
    #[prost(message, optional, tag="4")]
    pub registry_auth: ::core::option::Option<RegistryAuth>,
    #[prost(map="string, enumeration(task::RequiredParentStatus)", tag="5")]
    pub depends_on: ::std::collections::HashMap<::prost::alloc::string::String, i32>,
    #[prost(message, repeated, tag="6")]
    pub variables: ::prost::alloc::vec::Vec<Variable>,
    #[prost(message, optional, tag="7")]
    pub exec: ::core::option::Option<Exec>,
}
/// Nested message and enum types in `Task`.
pub mod task {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum RequiredParentStatus {
        Unknown = 0,
        Any = 1,
        Success = 2,
        Failure = 3,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PipelineTriggerSettings {
    #[prost(string, tag="1")]
    pub kind: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub label: ::prost::alloc::string::String,
    #[prost(map="string, string", tag="3")]
    pub settings: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
    #[prost(string, tag="4")]
    pub error: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PipelineNotifierSettings {
    #[prost(string, tag="1")]
    pub kind: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub label: ::prost::alloc::string::String,
    #[prost(map="string, string", tag="3")]
    pub settings: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
    #[prost(string, tag="4")]
    pub error: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TaskConfig {
    #[prost(string, tag="1")]
    pub id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub description: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub image: ::prost::alloc::string::String,
    #[prost(message, optional, tag="4")]
    pub registry_auth: ::core::option::Option<RegistryAuth>,
    #[prost(map="string, enumeration(task_config::RequiredParentStatus)", tag="5")]
    pub depends_on: ::std::collections::HashMap<::prost::alloc::string::String, i32>,
    #[prost(map="string, string", tag="6")]
    pub variables: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
    #[prost(message, optional, tag="7")]
    pub exec: ::core::option::Option<Exec>,
}
/// Nested message and enum types in `TaskConfig`.
pub mod task_config {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum RequiredParentStatus {
        Unknown = 0,
        Any = 1,
        Success = 2,
        Failure = 3,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PipelineTriggerConfig {
    #[prost(string, tag="1")]
    pub kind: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub label: ::prost::alloc::string::String,
    #[prost(map="string, string", tag="3")]
    pub settings: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PipelineNotifierConfig {
    #[prost(string, tag="1")]
    pub kind: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub label: ::prost::alloc::string::String,
    #[prost(map="string, string", tag="3")]
    pub settings: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TaskRunFailure {
    #[prost(enumeration="task_run_failure::Kind", tag="1")]
    pub kind: i32,
    #[prost(string, tag="2")]
    pub description: ::prost::alloc::string::String,
}
/// Nested message and enum types in `TaskRunFailure`.
pub mod task_run_failure {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Kind {
        Unknown = 0,
        AbnormalExit = 1,
        SchedulerError = 2,
        FailedPrecondition = 3,
        Cancelled = 4,
        Orphaned = 5,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TaskRun {
    #[prost(uint64, tag="1")]
    pub created: u64,
    #[prost(uint64, tag="2")]
    pub ended: u64,
    #[prost(uint64, tag="3")]
    pub exit_code: u64,
    #[prost(message, optional, tag="4")]
    pub failure: ::core::option::Option<TaskRunFailure>,
    #[prost(string, tag="5")]
    pub id: ::prost::alloc::string::String,
    #[prost(bool, tag="6")]
    pub logs_expired: bool,
    #[prost(bool, tag="7")]
    pub logs_removed: bool,
    #[prost(string, tag="8")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="9")]
    pub pipeline_id: ::prost::alloc::string::String,
    #[prost(uint64, tag="10")]
    pub run_id: u64,
    #[prost(string, tag="11")]
    pub scheduler_id: ::prost::alloc::string::String,
    #[prost(uint64, tag="12")]
    pub started: u64,
    #[prost(enumeration="task_run::State", tag="13")]
    pub state: i32,
    #[prost(enumeration="task_run::Status", tag="14")]
    pub status: i32,
    #[prost(message, optional, tag="15")]
    pub task: ::core::option::Option<Task>,
}
/// Nested message and enum types in `TaskRun`.
pub mod task_run {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum State {
        UnknownState = 0,
        Processing = 1,
        Running = 3,
        Complete = 4,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Status {
        UnknownStatus = 0,
        Successful = 1,
        Failed = 2,
        Cancelled = 3,
        Skipped = 4,
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum VariableOwner {
    Unknown = 0,
    User = 1,
    System = 2,
}
////////////// System Transport Models //////////////

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetSystemInfoRequest {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetSystemInfoResponse {
    #[prost(string, tag="1")]
    pub commit: ::prost::alloc::string::String,
    #[prost(bool, tag="2")]
    pub dev_mode_enabled: bool,
    #[prost(string, tag="3")]
    pub semver: ::prost::alloc::string::String,
}
////////////// Namespace Transport Models //////////////

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetNamespaceRequest {
    /// Unique identifier
    #[prost(string, tag="1")]
    pub id: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetNamespaceResponse {
    #[prost(message, optional, tag="1")]
    pub namespace: ::core::option::Option<Namespace>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListNamespacesRequest {
    /// offset is a pagination parameter that defines where to start when counting
    /// the list of objects to return.
    #[prost(uint64, tag="1")]
    pub offset: u64,
    /// limit is a pagination parameter that defines how many objects to return
    /// per result.
    #[prost(uint64, tag="2")]
    pub limit: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListNamespacesResponse {
    #[prost(message, repeated, tag="1")]
    pub namespaces: ::prost::alloc::vec::Vec<Namespace>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateNamespaceRequest {
    #[prost(string, tag="1")]
    pub id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub description: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateNamespaceResponse {
    #[prost(message, optional, tag="1")]
    pub namespace: ::core::option::Option<Namespace>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateNamespaceRequest {
    #[prost(string, tag="1")]
    pub id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub description: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateNamespaceResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteNamespaceRequest {
    /// Unique identifier
    #[prost(string, tag="1")]
    pub id: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteNamespaceResponse {
}
////////////// Pipeline Transport Models //////////////

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetPipelineRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    /// Unique identifier
    #[prost(string, tag="2")]
    pub id: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetPipelineResponse {
    #[prost(message, optional, tag="1")]
    pub pipeline: ::core::option::Option<Pipeline>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListPipelinesRequest {
    /// offset is a pagination parameter that defines where to start when counting
    /// the list of pipelines to return.
    #[prost(uint64, tag="1")]
    pub offset: u64,
    /// limit is a pagination parameter that defines how many pipelines to return
    /// per result.
    #[prost(uint64, tag="2")]
    pub limit: u64,
    /// Unique namespace identifier
    #[prost(string, tag="3")]
    pub namespace_id: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListPipelinesResponse {
    #[prost(message, repeated, tag="1")]
    pub pipelines: ::prost::alloc::vec::Vec<Pipeline>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RunPipelineRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub id: ::prost::alloc::string::String,
    /// variables allows for the replacement of task environment variables, it
    /// overrides all other environment variables if there is a name collision.
    #[prost(map="string, string", tag="3")]
    pub variables: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RunPipelineResponse {
    #[prost(message, optional, tag="1")]
    pub run: ::core::option::Option<Run>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DisablePipelineRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    /// Unique namespace identifier
    #[prost(string, tag="2")]
    pub id: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DisablePipelineResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EnablePipelineRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    /// Unique identifier
    #[prost(string, tag="2")]
    pub id: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EnablePipelineResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreatePipelineRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(message, optional, tag="2")]
    pub pipeline_config: ::core::option::Option<PipelineConfig>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreatePipelineResponse {
    #[prost(message, optional, tag="1")]
    pub pipeline: ::core::option::Option<Pipeline>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdatePipelineRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(message, optional, tag="2")]
    pub pipeline_config: ::core::option::Option<PipelineConfig>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdatePipelineResponse {
    #[prost(message, optional, tag="1")]
    pub pipeline: ::core::option::Option<Pipeline>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeletePipelineRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    /// Pipeline ID
    #[prost(string, tag="2")]
    pub id: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeletePipelineResponse {
}
////////////// Runs Transport Models //////////////

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetRunRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    /// Run ID
    #[prost(uint64, tag="3")]
    pub id: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetRunResponse {
    #[prost(message, optional, tag="1")]
    pub run: ::core::option::Option<Run>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BatchGetRunsRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    /// Run IDs
    #[prost(uint64, repeated, tag="3")]
    pub ids: ::prost::alloc::vec::Vec<u64>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BatchGetRunsResponse {
    #[prost(message, repeated, tag="1")]
    pub runs: ::prost::alloc::vec::Vec<Run>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListRunsRequest {
    /// offset is a pagination parameter that defines where to start when
    /// counting the list of pipelines to return
    #[prost(uint64, tag="1")]
    pub offset: u64,
    /// limit is a pagination parameter that defines how many pipelines to return
    /// per result.
    #[prost(uint64, tag="2")]
    pub limit: u64,
    /// Unique namespace identifier
    #[prost(string, tag="3")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="4")]
    pub pipeline_id: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListRunsResponse {
    #[prost(message, repeated, tag="1")]
    pub runs: ::prost::alloc::vec::Vec<Run>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StartRunRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    /// variables allows for the replacement of task environment variables, it
    /// overrides all other environment variables if there is a name collision.
    #[prost(map="string, string", tag="3")]
    pub variables: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StartRunResponse {
    #[prost(message, optional, tag="1")]
    pub run: ::core::option::Option<Run>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RetryRunRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    /// Run ID
    #[prost(uint64, tag="3")]
    pub id: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RetryRunResponse {
    #[prost(message, optional, tag="1")]
    pub run: ::core::option::Option<Run>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CancelRunRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    /// Run ID
    #[prost(uint64, tag="3")]
    pub id: u64,
    /// force will cause Gofer to hard kill any outstanding task run containers.
    /// Usually this means that the container receives a SIGKILL.
    #[prost(bool, tag="4")]
    pub force: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CancelRunResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CancelAllRunsRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    /// force will cause Gofer to hard kill any outstanding task run containers.
    /// Usually this means that the container receives a SIGKILL.
    #[prost(bool, tag="3")]
    pub force: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CancelAllRunsResponse {
    #[prost(uint64, repeated, tag="1")]
    pub runs: ::prost::alloc::vec::Vec<u64>,
}
////////////// Task Run Transport Models //////////////

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListTaskRunsRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    #[prost(uint64, tag="3")]
    pub run_id: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListTaskRunsResponse {
    #[prost(message, repeated, tag="1")]
    pub task_runs: ::prost::alloc::vec::Vec<TaskRun>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetTaskRunRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    #[prost(uint64, tag="3")]
    pub run_id: u64,
    /// Task Run ID
    #[prost(string, tag="4")]
    pub id: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetTaskRunResponse {
    #[prost(message, optional, tag="1")]
    pub task_run: ::core::option::Option<TaskRun>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CancelTaskRunRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    #[prost(uint64, tag="3")]
    pub run_id: u64,
    /// Task Run ID
    #[prost(string, tag="4")]
    pub id: ::prost::alloc::string::String,
    /// force will cause Gofer to hard kill this task run containers.
    /// Usually this means that the container receives a SIGKILL.
    #[prost(bool, tag="5")]
    pub force: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CancelTaskRunResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetTaskRunLogsRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    #[prost(uint64, tag="3")]
    pub run_id: u64,
    /// Task Run ID
    #[prost(string, tag="4")]
    pub id: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetTaskRunLogsResponse {
    /// The string content of the current log line.
    #[prost(string, tag="1")]
    pub log_line: ::prost::alloc::string::String,
    /// The current line number.
    #[prost(uint64, tag="2")]
    pub line_num: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteTaskRunLogsRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    #[prost(uint64, tag="3")]
    pub run_id: u64,
    /// Task Run ID
    #[prost(string, tag="4")]
    pub id: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteTaskRunLogsResponse {
}
/// Generated client implementations.
pub mod gofer_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    #[derive(Debug, Clone)]
    pub struct GoferClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl GoferClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: std::convert::TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> GoferClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> GoferClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            GoferClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with `gzip`.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_gzip(mut self) -> Self {
            self.inner = self.inner.send_gzip();
            self
        }
        /// Enable decompressing responses with `gzip`.
        #[must_use]
        pub fn accept_gzip(mut self) -> Self {
            self.inner = self.inner.accept_gzip();
            self
        }
        /////////////// System RPCs //////////////
        pub async fn get_system_info(
            &mut self,
            request: impl tonic::IntoRequest<super::GetSystemInfoRequest>,
        ) -> Result<tonic::Response<super::GetSystemInfoResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/proto.Gofer/GetSystemInfo",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// ListNamespaces returns all registered namespaces.
        pub async fn list_namespaces(
            &mut self,
            request: impl tonic::IntoRequest<super::ListNamespacesRequest>,
        ) -> Result<tonic::Response<super::ListNamespacesResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/proto.Gofer/ListNamespaces",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// CreateNamespace creates a new namespace that separates pipelines.
        pub async fn create_namespace(
            &mut self,
            request: impl tonic::IntoRequest<super::CreateNamespaceRequest>,
        ) -> Result<tonic::Response<super::CreateNamespaceResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/proto.Gofer/CreateNamespace",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// GetNamespace returns a single namespace by id.
        pub async fn get_namespace(
            &mut self,
            request: impl tonic::IntoRequest<super::GetNamespaceRequest>,
        ) -> Result<tonic::Response<super::GetNamespaceResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/GetNamespace");
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// UpdateNamespace updates the details of a particular namespace by id.
        pub async fn update_namespace(
            &mut self,
            request: impl tonic::IntoRequest<super::UpdateNamespaceRequest>,
        ) -> Result<tonic::Response<super::UpdateNamespaceResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/proto.Gofer/UpdateNamespace",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// DeleteNamespace removes a namespace by id.
        pub async fn delete_namespace(
            &mut self,
            request: impl tonic::IntoRequest<super::DeleteNamespaceRequest>,
        ) -> Result<tonic::Response<super::DeleteNamespaceResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/proto.Gofer/DeleteNamespace",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// GetPipeline returns a single pipeline by ID.
        pub async fn get_pipeline(
            &mut self,
            request: impl tonic::IntoRequest<super::GetPipelineRequest>,
        ) -> Result<tonic::Response<super::GetPipelineResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/GetPipeline");
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// ListPipelines returns all registered pipelines. Can control pagination by
        /// offset && limit request parameters.
        /// By default ListPipelines will return the first 100 pipelines ordered by
        /// creation.
        pub async fn list_pipelines(
            &mut self,
            request: impl tonic::IntoRequest<super::ListPipelinesRequest>,
        ) -> Result<tonic::Response<super::ListPipelinesResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/proto.Gofer/ListPipelines",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// RunPipeline executes a single run of this pipeline.
        pub async fn run_pipeline(
            &mut self,
            request: impl tonic::IntoRequest<super::RunPipelineRequest>,
        ) -> Result<tonic::Response<super::RunPipelineResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/RunPipeline");
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// EnablePipeline allows a pipeline to execute runs by allowing it to receive
        /// trigger events. See DisablePipeline to prevent a pipeline from executing
        /// any more runs.
        pub async fn enable_pipeline(
            &mut self,
            request: impl tonic::IntoRequest<super::EnablePipelineRequest>,
        ) -> Result<tonic::Response<super::EnablePipelineResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/proto.Gofer/EnablePipeline",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// DisablePipeline prevents the pipeline from executing runs. Any trigger
        /// events that would normally cause the pipeline to be run are instead
        /// discarded.
        pub async fn disable_pipeline(
            &mut self,
            request: impl tonic::IntoRequest<super::DisablePipelineRequest>,
        ) -> Result<tonic::Response<super::DisablePipelineResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/proto.Gofer/DisablePipeline",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// CreatePipeline creates a new pipeline from the protobuf input. This is
        /// usually autogenerated from the command line tool.
        pub async fn create_pipeline(
            &mut self,
            request: impl tonic::IntoRequest<super::CreatePipelineRequest>,
        ) -> Result<tonic::Response<super::CreatePipelineResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/proto.Gofer/CreatePipeline",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// UpdatePipeline updates a pipeline from the protobuf input. This input is
        /// usually autogenerated from the command line tool.
        /// Updating a pipeline requires the pipeline to adhere
        /// to two constraints:
        ///    1) The pipeline must not have any current runs in progress.
        ///    2) The pipeline must be in a disabled state.
        pub async fn update_pipeline(
            &mut self,
            request: impl tonic::IntoRequest<super::UpdatePipelineRequest>,
        ) -> Result<tonic::Response<super::UpdatePipelineResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/proto.Gofer/UpdatePipeline",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// DeletePipeline deletes a pipeline permenantly. It is not recoverable.
        pub async fn delete_pipeline(
            &mut self,
            request: impl tonic::IntoRequest<super::DeletePipelineRequest>,
        ) -> Result<tonic::Response<super::DeletePipelineResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/proto.Gofer/DeletePipeline",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// GetRun returns the details of a single run.
        pub async fn get_run(
            &mut self,
            request: impl tonic::IntoRequest<super::GetRunRequest>,
        ) -> Result<tonic::Response<super::GetRunResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/GetRun");
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// BatchGetRuns returns multiple runs by ID.
        pub async fn batch_get_runs(
            &mut self,
            request: impl tonic::IntoRequest<super::BatchGetRunsRequest>,
        ) -> Result<tonic::Response<super::BatchGetRunsResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/BatchGetRuns");
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// ListRuns returns a list of all runs by Pipeline ID. Pagination can be
        /// controlled via the offset and limit parameters of the request.
        pub async fn list_runs(
            &mut self,
            request: impl tonic::IntoRequest<super::ListRunsRequest>,
        ) -> Result<tonic::Response<super::ListRunsResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/ListRuns");
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// RetryRun simply takes the vars and settings from a previous run and re-uses
        /// those to launch a new run. Useful for if you want the exact settings from a
        /// previous run.
        pub async fn retry_run(
            &mut self,
            request: impl tonic::IntoRequest<super::RetryRunRequest>,
        ) -> Result<tonic::Response<super::RetryRunResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/RetryRun");
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// CancelRun stops the execution of a run in progress. Any task runs that
        /// might have been running at the time Are ask to stop gracefully(SIGINT)
        /// unless the force parameter is used, in which case the task runs are stopped
        /// instantly(SIGKILL) and the run is cancelled.
        pub async fn cancel_run(
            &mut self,
            request: impl tonic::IntoRequest<super::CancelRunRequest>,
        ) -> Result<tonic::Response<super::CancelRunResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/CancelRun");
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// CancelAllRuns stops the execution of any in-progress runs for a specific
        /// pipeline by ID.
        pub async fn cancel_all_runs(
            &mut self,
            request: impl tonic::IntoRequest<super::CancelAllRunsRequest>,
        ) -> Result<tonic::Response<super::CancelAllRunsResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/proto.Gofer/CancelAllRuns",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// GetTaskRun returns the details of a single task run.
        pub async fn get_task_run(
            &mut self,
            request: impl tonic::IntoRequest<super::GetTaskRunRequest>,
        ) -> Result<tonic::Response<super::GetTaskRunResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/GetTaskRun");
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// ListTaskRuns returns all task runs for a current run by ID.
        pub async fn list_task_runs(
            &mut self,
            request: impl tonic::IntoRequest<super::ListTaskRunsRequest>,
        ) -> Result<tonic::Response<super::ListTaskRunsResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/ListTaskRuns");
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// CancelTaskRun cancels a specific task run, sending the related container a
        /// SIGINT signal. If the force flag is used we instead send the container a
        /// SIGKILL signal.
        ///
        /// Task runs that are cancelled can cause other downstream task runs to be
        /// skipped depending on those downstream task run dependencies.
        pub async fn cancel_task_run(
            &mut self,
            request: impl tonic::IntoRequest<super::CancelTaskRunRequest>,
        ) -> Result<tonic::Response<super::CancelTaskRunResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/proto.Gofer/CancelTaskRun",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// GetTaskRunLogs returns logs for a specific task run line by line in a
        /// stream. The logs are returns with both STDOUT and STDERR of the associated
        /// container combined.
        pub async fn get_task_run_logs(
            &mut self,
            request: impl tonic::IntoRequest<super::GetTaskRunLogsRequest>,
        ) -> Result<
            tonic::Response<tonic::codec::Streaming<super::GetTaskRunLogsResponse>>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/proto.Gofer/GetTaskRunLogs",
            );
            self.inner.server_streaming(request.into_request(), path, codec).await
        }
        /// DeleteTaskRunLogs removes a task run's associated log object. This is
        /// useful for if logs mistakenly contain sensitive data.
        pub async fn delete_task_run_logs(
            &mut self,
            request: impl tonic::IntoRequest<super::DeleteTaskRunLogsRequest>,
        ) -> Result<tonic::Response<super::DeleteTaskRunLogsResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/proto.Gofer/DeleteTaskRunLogs",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod gofer_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    ///Generated trait containing gRPC methods that should be implemented for use with GoferServer.
    #[async_trait]
    pub trait Gofer: Send + Sync + 'static {
        /////////////// System RPCs //////////////
        async fn get_system_info(
            &self,
            request: tonic::Request<super::GetSystemInfoRequest>,
        ) -> Result<tonic::Response<super::GetSystemInfoResponse>, tonic::Status>;
        /// ListNamespaces returns all registered namespaces.
        async fn list_namespaces(
            &self,
            request: tonic::Request<super::ListNamespacesRequest>,
        ) -> Result<tonic::Response<super::ListNamespacesResponse>, tonic::Status>;
        /// CreateNamespace creates a new namespace that separates pipelines.
        async fn create_namespace(
            &self,
            request: tonic::Request<super::CreateNamespaceRequest>,
        ) -> Result<tonic::Response<super::CreateNamespaceResponse>, tonic::Status>;
        /// GetNamespace returns a single namespace by id.
        async fn get_namespace(
            &self,
            request: tonic::Request<super::GetNamespaceRequest>,
        ) -> Result<tonic::Response<super::GetNamespaceResponse>, tonic::Status>;
        /// UpdateNamespace updates the details of a particular namespace by id.
        async fn update_namespace(
            &self,
            request: tonic::Request<super::UpdateNamespaceRequest>,
        ) -> Result<tonic::Response<super::UpdateNamespaceResponse>, tonic::Status>;
        /// DeleteNamespace removes a namespace by id.
        async fn delete_namespace(
            &self,
            request: tonic::Request<super::DeleteNamespaceRequest>,
        ) -> Result<tonic::Response<super::DeleteNamespaceResponse>, tonic::Status>;
        /// GetPipeline returns a single pipeline by ID.
        async fn get_pipeline(
            &self,
            request: tonic::Request<super::GetPipelineRequest>,
        ) -> Result<tonic::Response<super::GetPipelineResponse>, tonic::Status>;
        /// ListPipelines returns all registered pipelines. Can control pagination by
        /// offset && limit request parameters.
        /// By default ListPipelines will return the first 100 pipelines ordered by
        /// creation.
        async fn list_pipelines(
            &self,
            request: tonic::Request<super::ListPipelinesRequest>,
        ) -> Result<tonic::Response<super::ListPipelinesResponse>, tonic::Status>;
        /// RunPipeline executes a single run of this pipeline.
        async fn run_pipeline(
            &self,
            request: tonic::Request<super::RunPipelineRequest>,
        ) -> Result<tonic::Response<super::RunPipelineResponse>, tonic::Status>;
        /// EnablePipeline allows a pipeline to execute runs by allowing it to receive
        /// trigger events. See DisablePipeline to prevent a pipeline from executing
        /// any more runs.
        async fn enable_pipeline(
            &self,
            request: tonic::Request<super::EnablePipelineRequest>,
        ) -> Result<tonic::Response<super::EnablePipelineResponse>, tonic::Status>;
        /// DisablePipeline prevents the pipeline from executing runs. Any trigger
        /// events that would normally cause the pipeline to be run are instead
        /// discarded.
        async fn disable_pipeline(
            &self,
            request: tonic::Request<super::DisablePipelineRequest>,
        ) -> Result<tonic::Response<super::DisablePipelineResponse>, tonic::Status>;
        /// CreatePipeline creates a new pipeline from the protobuf input. This is
        /// usually autogenerated from the command line tool.
        async fn create_pipeline(
            &self,
            request: tonic::Request<super::CreatePipelineRequest>,
        ) -> Result<tonic::Response<super::CreatePipelineResponse>, tonic::Status>;
        /// UpdatePipeline updates a pipeline from the protobuf input. This input is
        /// usually autogenerated from the command line tool.
        /// Updating a pipeline requires the pipeline to adhere
        /// to two constraints:
        ///    1) The pipeline must not have any current runs in progress.
        ///    2) The pipeline must be in a disabled state.
        async fn update_pipeline(
            &self,
            request: tonic::Request<super::UpdatePipelineRequest>,
        ) -> Result<tonic::Response<super::UpdatePipelineResponse>, tonic::Status>;
        /// DeletePipeline deletes a pipeline permenantly. It is not recoverable.
        async fn delete_pipeline(
            &self,
            request: tonic::Request<super::DeletePipelineRequest>,
        ) -> Result<tonic::Response<super::DeletePipelineResponse>, tonic::Status>;
        /// GetRun returns the details of a single run.
        async fn get_run(
            &self,
            request: tonic::Request<super::GetRunRequest>,
        ) -> Result<tonic::Response<super::GetRunResponse>, tonic::Status>;
        /// BatchGetRuns returns multiple runs by ID.
        async fn batch_get_runs(
            &self,
            request: tonic::Request<super::BatchGetRunsRequest>,
        ) -> Result<tonic::Response<super::BatchGetRunsResponse>, tonic::Status>;
        /// ListRuns returns a list of all runs by Pipeline ID. Pagination can be
        /// controlled via the offset and limit parameters of the request.
        async fn list_runs(
            &self,
            request: tonic::Request<super::ListRunsRequest>,
        ) -> Result<tonic::Response<super::ListRunsResponse>, tonic::Status>;
        /// RetryRun simply takes the vars and settings from a previous run and re-uses
        /// those to launch a new run. Useful for if you want the exact settings from a
        /// previous run.
        async fn retry_run(
            &self,
            request: tonic::Request<super::RetryRunRequest>,
        ) -> Result<tonic::Response<super::RetryRunResponse>, tonic::Status>;
        /// CancelRun stops the execution of a run in progress. Any task runs that
        /// might have been running at the time Are ask to stop gracefully(SIGINT)
        /// unless the force parameter is used, in which case the task runs are stopped
        /// instantly(SIGKILL) and the run is cancelled.
        async fn cancel_run(
            &self,
            request: tonic::Request<super::CancelRunRequest>,
        ) -> Result<tonic::Response<super::CancelRunResponse>, tonic::Status>;
        /// CancelAllRuns stops the execution of any in-progress runs for a specific
        /// pipeline by ID.
        async fn cancel_all_runs(
            &self,
            request: tonic::Request<super::CancelAllRunsRequest>,
        ) -> Result<tonic::Response<super::CancelAllRunsResponse>, tonic::Status>;
        /// GetTaskRun returns the details of a single task run.
        async fn get_task_run(
            &self,
            request: tonic::Request<super::GetTaskRunRequest>,
        ) -> Result<tonic::Response<super::GetTaskRunResponse>, tonic::Status>;
        /// ListTaskRuns returns all task runs for a current run by ID.
        async fn list_task_runs(
            &self,
            request: tonic::Request<super::ListTaskRunsRequest>,
        ) -> Result<tonic::Response<super::ListTaskRunsResponse>, tonic::Status>;
        /// CancelTaskRun cancels a specific task run, sending the related container a
        /// SIGINT signal. If the force flag is used we instead send the container a
        /// SIGKILL signal.
        ///
        /// Task runs that are cancelled can cause other downstream task runs to be
        /// skipped depending on those downstream task run dependencies.
        async fn cancel_task_run(
            &self,
            request: tonic::Request<super::CancelTaskRunRequest>,
        ) -> Result<tonic::Response<super::CancelTaskRunResponse>, tonic::Status>;
        ///Server streaming response type for the GetTaskRunLogs method.
        type GetTaskRunLogsStream: futures_core::Stream<
                Item = Result<super::GetTaskRunLogsResponse, tonic::Status>,
            >
            + Send
            + 'static;
        /// GetTaskRunLogs returns logs for a specific task run line by line in a
        /// stream. The logs are returns with both STDOUT and STDERR of the associated
        /// container combined.
        async fn get_task_run_logs(
            &self,
            request: tonic::Request<super::GetTaskRunLogsRequest>,
        ) -> Result<tonic::Response<Self::GetTaskRunLogsStream>, tonic::Status>;
        /// DeleteTaskRunLogs removes a task run's associated log object. This is
        /// useful for if logs mistakenly contain sensitive data.
        async fn delete_task_run_logs(
            &self,
            request: tonic::Request<super::DeleteTaskRunLogsRequest>,
        ) -> Result<tonic::Response<super::DeleteTaskRunLogsResponse>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct GoferServer<T: Gofer> {
        inner: _Inner<T>,
        accept_compression_encodings: (),
        send_compression_encodings: (),
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Gofer> GoferServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for GoferServer<T>
    where
        T: Gofer,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/proto.Gofer/GetSystemInfo" => {
                    #[allow(non_camel_case_types)]
                    struct GetSystemInfoSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::GetSystemInfoRequest>
                    for GetSystemInfoSvc<T> {
                        type Response = super::GetSystemInfoResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetSystemInfoRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).get_system_info(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetSystemInfoSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/ListNamespaces" => {
                    #[allow(non_camel_case_types)]
                    struct ListNamespacesSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::ListNamespacesRequest>
                    for ListNamespacesSvc<T> {
                        type Response = super::ListNamespacesResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ListNamespacesRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).list_namespaces(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ListNamespacesSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/CreateNamespace" => {
                    #[allow(non_camel_case_types)]
                    struct CreateNamespaceSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::CreateNamespaceRequest>
                    for CreateNamespaceSvc<T> {
                        type Response = super::CreateNamespaceResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::CreateNamespaceRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).create_namespace(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = CreateNamespaceSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/GetNamespace" => {
                    #[allow(non_camel_case_types)]
                    struct GetNamespaceSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::GetNamespaceRequest>
                    for GetNamespaceSvc<T> {
                        type Response = super::GetNamespaceResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetNamespaceRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).get_namespace(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetNamespaceSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/UpdateNamespace" => {
                    #[allow(non_camel_case_types)]
                    struct UpdateNamespaceSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::UpdateNamespaceRequest>
                    for UpdateNamespaceSvc<T> {
                        type Response = super::UpdateNamespaceResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::UpdateNamespaceRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).update_namespace(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = UpdateNamespaceSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/DeleteNamespace" => {
                    #[allow(non_camel_case_types)]
                    struct DeleteNamespaceSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::DeleteNamespaceRequest>
                    for DeleteNamespaceSvc<T> {
                        type Response = super::DeleteNamespaceResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::DeleteNamespaceRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).delete_namespace(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = DeleteNamespaceSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/GetPipeline" => {
                    #[allow(non_camel_case_types)]
                    struct GetPipelineSvc<T: Gofer>(pub Arc<T>);
                    impl<T: Gofer> tonic::server::UnaryService<super::GetPipelineRequest>
                    for GetPipelineSvc<T> {
                        type Response = super::GetPipelineResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetPipelineRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).get_pipeline(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetPipelineSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/ListPipelines" => {
                    #[allow(non_camel_case_types)]
                    struct ListPipelinesSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::ListPipelinesRequest>
                    for ListPipelinesSvc<T> {
                        type Response = super::ListPipelinesResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ListPipelinesRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).list_pipelines(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ListPipelinesSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/RunPipeline" => {
                    #[allow(non_camel_case_types)]
                    struct RunPipelineSvc<T: Gofer>(pub Arc<T>);
                    impl<T: Gofer> tonic::server::UnaryService<super::RunPipelineRequest>
                    for RunPipelineSvc<T> {
                        type Response = super::RunPipelineResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::RunPipelineRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).run_pipeline(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = RunPipelineSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/EnablePipeline" => {
                    #[allow(non_camel_case_types)]
                    struct EnablePipelineSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::EnablePipelineRequest>
                    for EnablePipelineSvc<T> {
                        type Response = super::EnablePipelineResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::EnablePipelineRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).enable_pipeline(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = EnablePipelineSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/DisablePipeline" => {
                    #[allow(non_camel_case_types)]
                    struct DisablePipelineSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::DisablePipelineRequest>
                    for DisablePipelineSvc<T> {
                        type Response = super::DisablePipelineResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::DisablePipelineRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).disable_pipeline(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = DisablePipelineSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/CreatePipeline" => {
                    #[allow(non_camel_case_types)]
                    struct CreatePipelineSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::CreatePipelineRequest>
                    for CreatePipelineSvc<T> {
                        type Response = super::CreatePipelineResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::CreatePipelineRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).create_pipeline(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = CreatePipelineSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/UpdatePipeline" => {
                    #[allow(non_camel_case_types)]
                    struct UpdatePipelineSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::UpdatePipelineRequest>
                    for UpdatePipelineSvc<T> {
                        type Response = super::UpdatePipelineResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::UpdatePipelineRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).update_pipeline(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = UpdatePipelineSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/DeletePipeline" => {
                    #[allow(non_camel_case_types)]
                    struct DeletePipelineSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::DeletePipelineRequest>
                    for DeletePipelineSvc<T> {
                        type Response = super::DeletePipelineResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::DeletePipelineRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).delete_pipeline(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = DeletePipelineSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/GetRun" => {
                    #[allow(non_camel_case_types)]
                    struct GetRunSvc<T: Gofer>(pub Arc<T>);
                    impl<T: Gofer> tonic::server::UnaryService<super::GetRunRequest>
                    for GetRunSvc<T> {
                        type Response = super::GetRunResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetRunRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).get_run(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetRunSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/BatchGetRuns" => {
                    #[allow(non_camel_case_types)]
                    struct BatchGetRunsSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::BatchGetRunsRequest>
                    for BatchGetRunsSvc<T> {
                        type Response = super::BatchGetRunsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::BatchGetRunsRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).batch_get_runs(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = BatchGetRunsSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/ListRuns" => {
                    #[allow(non_camel_case_types)]
                    struct ListRunsSvc<T: Gofer>(pub Arc<T>);
                    impl<T: Gofer> tonic::server::UnaryService<super::ListRunsRequest>
                    for ListRunsSvc<T> {
                        type Response = super::ListRunsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ListRunsRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).list_runs(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ListRunsSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/RetryRun" => {
                    #[allow(non_camel_case_types)]
                    struct RetryRunSvc<T: Gofer>(pub Arc<T>);
                    impl<T: Gofer> tonic::server::UnaryService<super::RetryRunRequest>
                    for RetryRunSvc<T> {
                        type Response = super::RetryRunResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::RetryRunRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).retry_run(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = RetryRunSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/CancelRun" => {
                    #[allow(non_camel_case_types)]
                    struct CancelRunSvc<T: Gofer>(pub Arc<T>);
                    impl<T: Gofer> tonic::server::UnaryService<super::CancelRunRequest>
                    for CancelRunSvc<T> {
                        type Response = super::CancelRunResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::CancelRunRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).cancel_run(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = CancelRunSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/CancelAllRuns" => {
                    #[allow(non_camel_case_types)]
                    struct CancelAllRunsSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::CancelAllRunsRequest>
                    for CancelAllRunsSvc<T> {
                        type Response = super::CancelAllRunsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::CancelAllRunsRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).cancel_all_runs(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = CancelAllRunsSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/GetTaskRun" => {
                    #[allow(non_camel_case_types)]
                    struct GetTaskRunSvc<T: Gofer>(pub Arc<T>);
                    impl<T: Gofer> tonic::server::UnaryService<super::GetTaskRunRequest>
                    for GetTaskRunSvc<T> {
                        type Response = super::GetTaskRunResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetTaskRunRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).get_task_run(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetTaskRunSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/ListTaskRuns" => {
                    #[allow(non_camel_case_types)]
                    struct ListTaskRunsSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::ListTaskRunsRequest>
                    for ListTaskRunsSvc<T> {
                        type Response = super::ListTaskRunsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ListTaskRunsRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).list_task_runs(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ListTaskRunsSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/CancelTaskRun" => {
                    #[allow(non_camel_case_types)]
                    struct CancelTaskRunSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::CancelTaskRunRequest>
                    for CancelTaskRunSvc<T> {
                        type Response = super::CancelTaskRunResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::CancelTaskRunRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).cancel_task_run(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = CancelTaskRunSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/GetTaskRunLogs" => {
                    #[allow(non_camel_case_types)]
                    struct GetTaskRunLogsSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::ServerStreamingService<super::GetTaskRunLogsRequest>
                    for GetTaskRunLogsSvc<T> {
                        type Response = super::GetTaskRunLogsResponse;
                        type ResponseStream = T::GetTaskRunLogsStream;
                        type Future = BoxFuture<
                            tonic::Response<Self::ResponseStream>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetTaskRunLogsRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).get_task_run_logs(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetTaskRunLogsSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.server_streaming(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.Gofer/DeleteTaskRunLogs" => {
                    #[allow(non_camel_case_types)]
                    struct DeleteTaskRunLogsSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::DeleteTaskRunLogsRequest>
                    for DeleteTaskRunLogsSvc<T> {
                        type Response = super::DeleteTaskRunLogsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::DeleteTaskRunLogsRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).delete_task_run_logs(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = DeleteTaskRunLogsSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }
    impl<T: Gofer> Clone for GoferServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }
    impl<T: Gofer> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: Gofer> tonic::transport::NamedService for GoferServer<T> {
        const NAME: &'static str = "proto.Gofer";
    }
}
