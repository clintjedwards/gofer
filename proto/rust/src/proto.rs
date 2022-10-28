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
    #[prost(int64, tag="4")]
    pub created: i64,
    #[prost(int64, tag="5")]
    pub modified: i64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Variable {
    #[prost(string, tag="1")]
    pub key: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub value: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub source: ::prost::alloc::string::String,
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
    #[prost(int64, tag="5")]
    pub parallelism: i64,
    #[prost(int64, tag="6")]
    pub created: i64,
    #[prost(int64, tag="7")]
    pub modified: i64,
    #[prost(enumeration="pipeline::PipelineState", tag="8")]
    pub state: i32,
    #[prost(map="string, message", tag="9")]
    pub custom_tasks: ::std::collections::HashMap<::prost::alloc::string::String, CustomTask>,
    #[prost(map="string, message", tag="10")]
    pub triggers: ::std::collections::HashMap<::prost::alloc::string::String, PipelineTriggerSettings>,
    #[prost(map="string, message", tag="11")]
    pub common_tasks: ::std::collections::HashMap<::prost::alloc::string::String, PipelineCommonTaskSettings>,
    #[prost(message, repeated, tag="12")]
    pub errors: ::prost::alloc::vec::Vec<pipeline::Error>,
}
/// Nested message and enum types in `Pipeline`.
pub mod pipeline {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Error {
        #[prost(enumeration="ErrorKind", tag="1")]
        pub kind: i32,
        #[prost(string, tag="2")]
        pub description: ::prost::alloc::string::String,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum PipelineState {
        Unknown = 0,
        Active = 1,
        Disabled = 2,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum ErrorKind {
        Unknown = 0,
        TriggerSubscriptionFailure = 1,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PipelineTaskConfig {
    #[prost(oneof="pipeline_task_config::Task", tags="1, 2")]
    pub task: ::core::option::Option<pipeline_task_config::Task>,
}
/// Nested message and enum types in `PipelineTaskConfig`.
pub mod pipeline_task_config {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Task {
        #[prost(message, tag="1")]
        CustomTask(super::CustomTaskConfig),
        #[prost(message, tag="2")]
        CommonTask(super::CommonTaskConfig),
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
    #[prost(int64, tag="4")]
    pub parallelism: i64,
    #[prost(message, repeated, tag="5")]
    pub tasks: ::prost::alloc::vec::Vec<PipelineTaskConfig>,
    #[prost(message, repeated, tag="6")]
    pub triggers: ::prost::alloc::vec::Vec<PipelineTriggerConfig>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Run {
    #[prost(string, tag="1")]
    pub namespace: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline: ::prost::alloc::string::String,
    #[prost(int64, tag="3")]
    pub id: i64,
    #[prost(int64, tag="4")]
    pub started: i64,
    #[prost(int64, tag="5")]
    pub ended: i64,
    #[prost(enumeration="run::RunState", tag="6")]
    pub state: i32,
    #[prost(enumeration="run::RunStatus", tag="7")]
    pub status: i32,
    #[prost(message, optional, tag="8")]
    pub status_reason: ::core::option::Option<RunStatusReason>,
    #[prost(string, repeated, tag="9")]
    pub task_runs: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(message, optional, tag="10")]
    pub trigger: ::core::option::Option<run::RunTriggerInfo>,
    #[prost(message, repeated, tag="11")]
    pub variables: ::prost::alloc::vec::Vec<Variable>,
    #[prost(bool, tag="12")]
    pub store_objects_expired: bool,
}
/// Nested message and enum types in `Run`.
pub mod run {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct RunTriggerInfo {
        #[prost(string, tag="1")]
        pub name: ::prost::alloc::string::String,
        #[prost(string, tag="2")]
        pub label: ::prost::alloc::string::String,
    }
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
pub struct RunStatusReason {
    #[prost(enumeration="run_status_reason::RunStatusReasonKind", tag="1")]
    pub reason: i32,
    #[prost(string, tag="2")]
    pub description: ::prost::alloc::string::String,
}
/// Nested message and enum types in `RunStatusReason`.
pub mod run_status_reason {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum RunStatusReasonKind {
        RunStatusReasonUnknown = 0,
        AbnormalExit = 1,
        SchedulerError = 2,
        FailedPrecondition = 3,
        UserCancelled = 4,
        AdminCancelled = 5,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RegistryAuth {
    #[prost(string, tag="1")]
    pub user: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pass: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CustomTask {
    #[prost(string, tag="1")]
    pub id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub description: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub image: ::prost::alloc::string::String,
    #[prost(message, optional, tag="4")]
    pub registry_auth: ::core::option::Option<RegistryAuth>,
    #[prost(map="string, enumeration(custom_task::RequiredParentStatus)", tag="5")]
    pub depends_on: ::std::collections::HashMap<::prost::alloc::string::String, i32>,
    #[prost(message, repeated, tag="6")]
    pub variables: ::prost::alloc::vec::Vec<Variable>,
    #[prost(string, repeated, tag="7")]
    pub entrypoint: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(string, repeated, tag="8")]
    pub command: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
/// Nested message and enum types in `CustomTask`.
pub mod custom_task {
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
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub label: ::prost::alloc::string::String,
    #[prost(map="string, string", tag="3")]
    pub settings: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PipelineCommonTaskSettings {
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub label: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub description: ::prost::alloc::string::String,
    #[prost(map="string, enumeration(pipeline_common_task_settings::RequiredParentStatus)", tag="4")]
    pub depends_on: ::std::collections::HashMap<::prost::alloc::string::String, i32>,
    #[prost(map="string, string", tag="5")]
    pub settings: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
}
/// Nested message and enum types in `PipelineCommonTaskSettings`.
pub mod pipeline_common_task_settings {
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
pub struct CustomTaskConfig {
    #[prost(string, tag="1")]
    pub id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub description: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub image: ::prost::alloc::string::String,
    #[prost(message, optional, tag="4")]
    pub registry_auth: ::core::option::Option<RegistryAuth>,
    #[prost(map="string, enumeration(custom_task_config::RequiredParentStatus)", tag="5")]
    pub depends_on: ::std::collections::HashMap<::prost::alloc::string::String, i32>,
    #[prost(map="string, string", tag="6")]
    pub variables: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
    #[prost(string, repeated, tag="7")]
    pub entrypoint: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(string, repeated, tag="8")]
    pub command: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
/// Nested message and enum types in `CustomTaskConfig`.
pub mod custom_task_config {
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
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub label: ::prost::alloc::string::String,
    #[prost(map="string, string", tag="3")]
    pub settings: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CommonTask {
    #[prost(message, optional, tag="1")]
    pub settings: ::core::option::Option<PipelineCommonTaskSettings>,
    #[prost(message, optional, tag="2")]
    pub registration: ::core::option::Option<CommonTaskRegistration>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CommonTaskConfig {
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub label: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub description: ::prost::alloc::string::String,
    #[prost(map="string, enumeration(common_task_config::RequiredParentStatus)", tag="4")]
    pub depends_on: ::std::collections::HashMap<::prost::alloc::string::String, i32>,
    #[prost(map="string, string", tag="5")]
    pub settings: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
}
/// Nested message and enum types in `CommonTaskConfig`.
pub mod common_task_config {
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
pub struct TaskRunStatusReason {
    #[prost(enumeration="task_run_status_reason::Reason", tag="1")]
    pub reason: i32,
    #[prost(string, tag="2")]
    pub description: ::prost::alloc::string::String,
}
/// Nested message and enum types in `TaskRunStatusReason`.
pub mod task_run_status_reason {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Reason {
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
    #[prost(int64, tag="1")]
    pub created: i64,
    #[prost(int64, tag="2")]
    pub ended: i64,
    #[prost(int64, tag="3")]
    pub exit_code: i64,
    #[prost(message, optional, tag="4")]
    pub status_reason: ::core::option::Option<TaskRunStatusReason>,
    #[prost(string, tag="5")]
    pub id: ::prost::alloc::string::String,
    #[prost(bool, tag="6")]
    pub logs_expired: bool,
    #[prost(bool, tag="7")]
    pub logs_removed: bool,
    #[prost(string, tag="8")]
    pub namespace: ::prost::alloc::string::String,
    #[prost(string, tag="9")]
    pub pipeline: ::prost::alloc::string::String,
    #[prost(int64, tag="10")]
    pub run: i64,
    #[prost(int64, tag="11")]
    pub started: i64,
    #[prost(enumeration="task_run::TaskRunState", tag="12")]
    pub state: i32,
    #[prost(enumeration="task_run::TaskRunStatus", tag="13")]
    pub status: i32,
    #[prost(message, repeated, tag="16")]
    pub variables: ::prost::alloc::vec::Vec<Variable>,
    #[prost(oneof="task_run::Task", tags="14, 15")]
    pub task: ::core::option::Option<task_run::Task>,
}
/// Nested message and enum types in `TaskRun`.
pub mod task_run {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum TaskRunState {
        UnknownState = 0,
        Processing = 1,
        Waiting = 2,
        Running = 3,
        Complete = 4,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum TaskRunStatus {
        UnknownStatus = 0,
        Successful = 1,
        Failed = 2,
        Cancelled = 3,
        Skipped = 4,
    }
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Task {
        #[prost(message, tag="14")]
        CustomTask(super::CustomTask),
        #[prost(message, tag="15")]
        CommonTask(super::CommonTask),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Trigger {
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub image: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub url: ::prost::alloc::string::String,
    #[prost(int64, tag="4")]
    pub started: i64,
    #[prost(enumeration="trigger::TriggerState", tag="5")]
    pub state: i32,
    #[prost(enumeration="trigger::TriggerStatus", tag="6")]
    pub status: i32,
    #[prost(string, tag="7")]
    pub documentation: ::prost::alloc::string::String,
}
/// Nested message and enum types in `Trigger`.
pub mod trigger {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum TriggerState {
        UnknownState = 0,
        Processing = 1,
        Running = 2,
        Exited = 3,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum TriggerStatus {
        UnknownStatus = 0,
        Enabled = 1,
        Disabled = 2,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TriggerRegistration {
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub image: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub user: ::prost::alloc::string::String,
    #[prost(string, tag="4")]
    pub pass: ::prost::alloc::string::String,
    #[prost(map="string, string", tag="5")]
    pub variables: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
    #[prost(int64, tag="6")]
    pub created: i64,
    #[prost(enumeration="trigger_registration::TriggerStatus", tag="7")]
    pub status: i32,
}
/// Nested message and enum types in `TriggerRegistration`.
pub mod trigger_registration {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum TriggerStatus {
        UnknownStatus = 0,
        Enabled = 1,
        Disabled = 2,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CommonTaskRegistration {
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub image: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub user: ::prost::alloc::string::String,
    #[prost(string, tag="4")]
    pub pass: ::prost::alloc::string::String,
    #[prost(message, repeated, tag="5")]
    pub variables: ::prost::alloc::vec::Vec<Variable>,
    #[prost(int64, tag="6")]
    pub created: i64,
    #[prost(enumeration="common_task_registration::Status", tag="7")]
    pub status: i32,
    #[prost(string, tag="8")]
    pub documentation: ::prost::alloc::string::String,
}
/// Nested message and enum types in `CommonTaskRegistration`.
pub mod common_task_registration {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Status {
        Unknown = 0,
        Enabled = 1,
        Disabled = 2,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Event {
    #[prost(int64, tag="1")]
    pub id: i64,
    /// What type of event
    #[prost(string, tag="2")]
    pub kind: ::prost::alloc::string::String,
    /// Json output of the event
    #[prost(string, tag="3")]
    pub details: ::prost::alloc::string::String,
    #[prost(int64, tag="4")]
    pub emitted: i64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Token {
    #[prost(int64, tag="1")]
    pub created: i64,
    #[prost(enumeration="token::Kind", tag="2")]
    pub kind: i32,
    #[prost(string, repeated, tag="3")]
    pub namespaces: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(map="string, string", tag="4")]
    pub metadata: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
    #[prost(int64, tag="5")]
    pub expires: i64,
    #[prost(bool, tag="6")]
    pub disabled: bool,
}
/// Nested message and enum types in `Token`.
pub mod token {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Kind {
        Unknown = 0,
        Management = 1,
        Client = 2,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TriggerResult {
    #[prost(enumeration="trigger_result::Status", tag="1")]
    pub status: i32,
    #[prost(string, tag="2")]
    pub details: ::prost::alloc::string::String,
}
/// Nested message and enum types in `TriggerResult`.
pub mod trigger_result {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Status {
        Unknown = 0,
        Failure = 1,
        Success = 2,
        Skipped = 3,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SecretStoreKey {
    #[prost(string, tag="1")]
    pub key: ::prost::alloc::string::String,
    #[prost(int64, tag="2")]
    pub created: i64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ObjectStoreKey {
    #[prost(string, tag="1")]
    pub key: ::prost::alloc::string::String,
    #[prost(int64, tag="2")]
    pub created: i64,
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
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RepairOrphanRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    #[prost(int64, tag="3")]
    pub run_id: i64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RepairOrphanResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ToggleEventIngressRequest {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ToggleEventIngressResponse {
    /// The current value for the boolean that controls event ingress.
    #[prost(bool, tag="1")]
    pub value: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateTokenRequest {
    #[prost(enumeration="create_token_request::Kind", tag="1")]
    pub kind: i32,
    #[prost(string, repeated, tag="2")]
    pub namespaces: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(map="string, string", tag="3")]
    pub metadata: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
    /// Accepts golang duration strings
    /// <https://pkg.go.dev/time#ParseDuration>
    #[prost(string, tag="4")]
    pub expires: ::prost::alloc::string::String,
}
/// Nested message and enum types in `CreateTokenRequest`.
pub mod create_token_request {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Kind {
        Unknown = 0,
        Management = 1,
        Client = 2,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateTokenResponse {
    #[prost(message, optional, tag="1")]
    pub details: ::core::option::Option<Token>,
    #[prost(string, tag="2")]
    pub token: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BootstrapTokenRequest {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BootstrapTokenResponse {
    #[prost(message, optional, tag="1")]
    pub details: ::core::option::Option<Token>,
    #[prost(string, tag="2")]
    pub token: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetTokenRequest {
    #[prost(string, tag="1")]
    pub token: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetTokenResponse {
    #[prost(message, optional, tag="1")]
    pub details: ::core::option::Option<Token>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListTokensRequest {
    #[prost(string, tag="1")]
    pub namespace: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListTokensResponse {
    #[prost(message, repeated, tag="1")]
    pub tokens: ::prost::alloc::vec::Vec<Token>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteTokenRequest {
    #[prost(string, tag="1")]
    pub token: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteTokenResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EnableTokenRequest {
    #[prost(string, tag="1")]
    pub token: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EnableTokenResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DisableTokenRequest {
    #[prost(string, tag="1")]
    pub token: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DisableTokenResponse {
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
    #[prost(int64, tag="1")]
    pub offset: i64,
    /// limit is a pagination parameter that defines how many objects to return
    /// per result.
    #[prost(int64, tag="2")]
    pub limit: i64,
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
    #[prost(int64, tag="1")]
    pub offset: i64,
    /// limit is a pagination parameter that defines how many pipelines to return
    /// per result.
    #[prost(int64, tag="2")]
    pub limit: i64,
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
    #[prost(int64, tag="3")]
    pub id: i64,
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
    #[prost(int64, repeated, tag="3")]
    pub ids: ::prost::alloc::vec::Vec<i64>,
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
    #[prost(int64, tag="1")]
    pub offset: i64,
    /// limit is a pagination parameter that defines how many pipelines to return
    /// per result.
    #[prost(int64, tag="2")]
    pub limit: i64,
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
    #[prost(int64, tag="3")]
    pub run_id: i64,
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
    #[prost(int64, tag="3")]
    pub run_id: i64,
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
    #[prost(int64, repeated, tag="1")]
    pub runs: ::prost::alloc::vec::Vec<i64>,
}
////////////// Task Run Transport Models //////////////

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListTaskRunsRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    #[prost(int64, tag="3")]
    pub run_id: i64,
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
    #[prost(int64, tag="3")]
    pub run_id: i64,
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
    #[prost(int64, tag="3")]
    pub run_id: i64,
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
    #[prost(int64, tag="3")]
    pub run_id: i64,
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
    #[prost(int64, tag="2")]
    pub line_num: i64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteTaskRunLogsRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    #[prost(int64, tag="3")]
    pub run_id: i64,
    /// Task Run ID
    #[prost(string, tag="4")]
    pub id: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteTaskRunLogsResponse {
}
////////////// Trigger Transport Models //////////////

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetTriggerRequest {
    /// The unique name for a particular trigger
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetTriggerResponse {
    #[prost(message, optional, tag="1")]
    pub trigger: ::core::option::Option<Trigger>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListTriggersRequest {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListTriggersResponse {
    #[prost(message, repeated, tag="1")]
    pub triggers: ::prost::alloc::vec::Vec<Trigger>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InstallTriggerRequest {
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub image: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub user: ::prost::alloc::string::String,
    #[prost(string, tag="4")]
    pub pass: ::prost::alloc::string::String,
    #[prost(map="string, string", tag="5")]
    pub variables: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InstallTriggerResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UninstallTriggerRequest {
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UninstallTriggerResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EnableTriggerRequest {
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EnableTriggerResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DisableTriggerRequest {
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DisableTriggerResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetTriggerInstallInstructionsRequest {
    #[prost(string, tag="1")]
    pub image: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub user: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub pass: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetTriggerInstallInstructionsResponse {
    #[prost(string, tag="1")]
    pub instructions: ::prost::alloc::string::String,
}
////////////// CommonTask Transport Models //////////////

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetCommonTaskRequest {
    /// The unique name/kind for a particular commontask
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetCommonTaskResponse {
    #[prost(message, optional, tag="1")]
    pub common_task: ::core::option::Option<CommonTaskRegistration>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListCommonTasksRequest {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListCommonTasksResponse {
    #[prost(message, repeated, tag="1")]
    pub common_tasks: ::prost::alloc::vec::Vec<CommonTaskRegistration>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InstallCommonTaskRequest {
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub image: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub user: ::prost::alloc::string::String,
    #[prost(string, tag="4")]
    pub pass: ::prost::alloc::string::String,
    #[prost(map="string, string", tag="5")]
    pub variables: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
    #[prost(string, tag="6")]
    pub documentation: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InstallCommonTaskResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UninstallCommonTaskRequest {
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UninstallCommonTaskResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EnableCommonTaskRequest {
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EnableCommonTaskResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DisableCommonTaskRequest {
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DisableCommonTaskResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetCommonTaskInstallInstructionsRequest {
    #[prost(string, tag="1")]
    pub image: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub user: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub pass: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetCommonTaskInstallInstructionsResponse {
    #[prost(string, tag="1")]
    pub instructions: ::prost::alloc::string::String,
}
////////////// Trigger Service Transport Models //////////////

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TriggerWatchRequest {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TriggerWatchResponse {
    /// The trigger can choose to give extra details about the specific trigger
    /// event result in the form of a string description.
    #[prost(string, tag="1")]
    pub details: ::prost::alloc::string::String,
    /// Unique identifier for namespace.
    #[prost(string, tag="2")]
    pub namespace_id: ::prost::alloc::string::String,
    /// Unique identifier for pipeline.
    #[prost(string, tag="3")]
    pub pipeline_id: ::prost::alloc::string::String,
    /// Unique id of trigger instance.
    #[prost(string, tag="4")]
    pub pipeline_trigger_label: ::prost::alloc::string::String,
    #[prost(enumeration="trigger_watch_response::Result", tag="5")]
    pub result: i32,
    /// Metadata is passed to the tasks as extra environment variables.
    #[prost(map="string, string", tag="6")]
    pub metadata: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
}
/// Nested message and enum types in `TriggerWatchResponse`.
pub mod trigger_watch_response {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Result {
        Unknown = 0,
        Success = 1,
        Failure = 2,
        Skipped = 3,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TriggerInfoRequest {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TriggerInfoResponse {
    /// kind corresponds a unique trigger identifier, this is passed as a envvar
    /// via the main process(and as such can be left empty), as the main process
    /// container the configuration for which trigger "kind" corresponds to which
    /// trigger container.
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
    /// Triggers are allowed to provide a link to more extensive documentation on
    /// how to use and configure them.
    #[prost(string, tag="2")]
    pub documentation: ::prost::alloc::string::String,
    /// A listing of all registered pipelines in the format: <namespace>/<pipeline>
    #[prost(string, repeated, tag="3")]
    pub registered: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TriggerSubscribeRequest {
    /// unique identifier for associated namespace
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    /// unique identifier for associated pipeline
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    /// pipeline specific subscription id
    #[prost(string, tag="3")]
    pub pipeline_trigger_label: ::prost::alloc::string::String,
    /// Pipelines are allowed to pass a configuration to triggers denoting what
    /// specific settings they might like for a specific trigger. The acceptable
    /// values of this config map is defined by the triggers and should be
    /// mentioned in documentation.
    ///
    /// Additionally, the trigger should verify config settings and pass back an
    /// error when it does not meet requirements.
    ///
    /// Note: The keys in this map are forced to be uppercase. This is important
    /// when checking for their existance when writing a trigger.
    #[prost(map="string, string", tag="4")]
    pub config: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TriggerSubscribeResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TriggerUnsubscribeRequest {
    /// unique identifier for associated namespace
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    /// unique identifier for associated pipeline
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    /// pipeline specific subscription id
    #[prost(string, tag="3")]
    pub pipeline_trigger_label: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TriggerUnsubscribeResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TriggerShutdownRequest {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TriggerShutdownResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TriggerExternalEventRequest {
    #[prost(bytes="vec", tag="1")]
    pub payload: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TriggerExternalEventResponse {
}
////////////// Events Transport Models //////////////

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetEventRequest {
    #[prost(int64, tag="1")]
    pub id: i64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetEventResponse {
    #[prost(message, optional, tag="1")]
    pub event: ::core::option::Option<Event>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListEventsRequest {
    /// defaults to false; meaning oldest to newest events by default.
    #[prost(bool, tag="1")]
    pub reverse: bool,
    /// Tell Gofer to continually stream new events instead of closing the stream
    /// after it gets to the end.
    #[prost(bool, tag="2")]
    pub follow: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListEventsResponse {
    #[prost(message, optional, tag="1")]
    pub event: ::core::option::Option<Event>,
}
////////////// Object store Transport Models //////////////

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetPipelineObjectRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub key: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetPipelineObjectResponse {
    #[prost(bytes="vec", tag="1")]
    pub content: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListPipelineObjectsRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListPipelineObjectsResponse {
    #[prost(message, repeated, tag="1")]
    pub keys: ::prost::alloc::vec::Vec<ObjectStoreKey>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PutPipelineObjectRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub key: ::prost::alloc::string::String,
    #[prost(bytes="vec", tag="4")]
    pub content: ::prost::alloc::vec::Vec<u8>,
    /// Overwrites an already existing value.
    #[prost(bool, tag="5")]
    pub force: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PutPipelineObjectResponse {
    /// The number of bytes uploaded.
    #[prost(int64, tag="1")]
    pub bytes: i64,
    /// The total amount of objects for this particular pipeline.
    #[prost(int64, tag="2")]
    pub object_limit: i64,
    /// The key for the object that was evicted for the latest key.
    #[prost(string, tag="3")]
    pub object_evicted: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeletePipelineObjectRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub key: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeletePipelineObjectResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetRunObjectRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    #[prost(int64, tag="3")]
    pub run_id: i64,
    #[prost(string, tag="4")]
    pub key: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetRunObjectResponse {
    #[prost(bytes="vec", tag="1")]
    pub content: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListRunObjectsRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    #[prost(int64, tag="3")]
    pub run_id: i64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListRunObjectsResponse {
    #[prost(message, repeated, tag="1")]
    pub keys: ::prost::alloc::vec::Vec<ObjectStoreKey>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PutRunObjectRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    #[prost(int64, tag="3")]
    pub run_id: i64,
    #[prost(string, tag="4")]
    pub key: ::prost::alloc::string::String,
    #[prost(bytes="vec", tag="5")]
    pub content: ::prost::alloc::vec::Vec<u8>,
    /// Overwrites an already existing value.
    #[prost(bool, tag="6")]
    pub force: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PutRunObjectResponse {
    #[prost(int64, tag="1")]
    pub bytes: i64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteRunObjectRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    #[prost(int64, tag="3")]
    pub run_id: i64,
    #[prost(string, tag="4")]
    pub key: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteRunObjectResponse {
}
////////////// Secret store Transport Models //////////////

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetPipelineSecretRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub key: ::prost::alloc::string::String,
    /// Whether to include plaintext secret
    #[prost(bool, tag="4")]
    pub include_secret: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetPipelineSecretResponse {
    #[prost(message, optional, tag="1")]
    pub metadata: ::core::option::Option<SecretStoreKey>,
    #[prost(string, tag="2")]
    pub secret: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListPipelineSecretsRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListPipelineSecretsResponse {
    #[prost(message, repeated, tag="1")]
    pub keys: ::prost::alloc::vec::Vec<SecretStoreKey>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PutPipelineSecretRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub key: ::prost::alloc::string::String,
    #[prost(string, tag="4")]
    pub content: ::prost::alloc::string::String,
    /// Overwrites an already existing value.
    #[prost(bool, tag="5")]
    pub force: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PutPipelineSecretResponse {
    /// The number of bytes uploaded.
    #[prost(int64, tag="1")]
    pub bytes: i64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeletePipelineSecretRequest {
    /// Unique namespace identifier
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub key: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeletePipelineSecretResponse {
}
////////////// Secret store Transport Models //////////////

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetGlobalSecretRequest {
    #[prost(string, tag="1")]
    pub key: ::prost::alloc::string::String,
    /// Whether to include plaintext secret
    #[prost(bool, tag="2")]
    pub include_secret: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetGlobalSecretResponse {
    #[prost(message, optional, tag="1")]
    pub metadata: ::core::option::Option<SecretStoreKey>,
    #[prost(string, tag="2")]
    pub secret: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListGlobalSecretsRequest {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListGlobalSecretsResponse {
    #[prost(message, repeated, tag="1")]
    pub keys: ::prost::alloc::vec::Vec<SecretStoreKey>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PutGlobalSecretRequest {
    #[prost(string, tag="1")]
    pub key: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub content: ::prost::alloc::string::String,
    /// Overwrites an already existing value.
    #[prost(bool, tag="3")]
    pub force: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PutGlobalSecretResponse {
    /// The number of bytes uploaded.
    #[prost(int64, tag="1")]
    pub bytes: i64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteGlobalSecretRequest {
    #[prost(string, tag="1")]
    pub key: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteGlobalSecretResponse {
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
        #[doc = "//////////// System RPCs //////////////"]
        ///
        /// Service RPCs exist to help with management of the Gofer service. They
        /// usually perform admin type interactions with the service as a whole and
        /// provide ways for admins to quickly repair Gofer broken states without
        /// having to stop the entire service.
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
        /// RepairOrphan is used when a single run has gotten into a state that does
        /// not reflect what actually happened to the run. This can happen if the Gofer
        /// service crashes for unforeseen reasons. Usually this route is not needed as
        /// Gofer will make an attempt to resolve all orphaned runs upon startup. But
        /// in the rare case that a run gets into a bad state during the service's
        /// normal execution this route can be used to attempt to repair the orphaned
        /// run or at the very least mark it as failed so it isn't stuck in a
        /// unfinished state.
        pub async fn repair_orphan(
            &mut self,
            request: impl tonic::IntoRequest<super::RepairOrphanRequest>,
        ) -> Result<tonic::Response<super::RepairOrphanResponse>, tonic::Status> {
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
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/RepairOrphan");
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// ToggleEventIngress allows the admin to start or stop the execution of all
        /// pipelines within Gofer. This can be useful under some security implications
        /// or for the purposes of defining general downtime and service maintenance.
        pub async fn toggle_event_ingress(
            &mut self,
            request: impl tonic::IntoRequest<super::ToggleEventIngressRequest>,
        ) -> Result<tonic::Response<super::ToggleEventIngressResponse>, tonic::Status> {
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
                "/proto.Gofer/ToggleEventIngress",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// CreateToken manifests a new API token; This token can be a management
        /// token(the equivalent of root in Linux) or a client token. Management tokens
        /// are the only tokens that can generate tokens.
        /// Client tokens are used to manage which namespaces users have access to.
        pub async fn create_token(
            &mut self,
            request: impl tonic::IntoRequest<super::CreateTokenRequest>,
        ) -> Result<tonic::Response<super::CreateTokenResponse>, tonic::Status> {
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
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/CreateToken");
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// BootstrapToken creates the initial management token used to create all
        /// other tokens.
        pub async fn bootstrap_token(
            &mut self,
            request: impl tonic::IntoRequest<super::BootstrapTokenRequest>,
        ) -> Result<tonic::Response<super::BootstrapTokenResponse>, tonic::Status> {
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
                "/proto.Gofer/BootstrapToken",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// ListTokens returns information about all tokens for a particular namespace;
        pub async fn list_tokens(
            &mut self,
            request: impl tonic::IntoRequest<super::ListTokensRequest>,
        ) -> Result<tonic::Response<super::ListTokensResponse>, tonic::Status> {
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
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/ListTokens");
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// GetToken returns information about a particular token;
        pub async fn get_token(
            &mut self,
            request: impl tonic::IntoRequest<super::GetTokenRequest>,
        ) -> Result<tonic::Response<super::GetTokenResponse>, tonic::Status> {
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
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/GetToken");
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// EnableToken makes a token usable.
        pub async fn enable_token(
            &mut self,
            request: impl tonic::IntoRequest<super::EnableTokenRequest>,
        ) -> Result<tonic::Response<super::EnableTokenResponse>, tonic::Status> {
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
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/EnableToken");
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// DisableToken makes a token unusable.
        pub async fn disable_token(
            &mut self,
            request: impl tonic::IntoRequest<super::DisableTokenRequest>,
        ) -> Result<tonic::Response<super::DisableTokenResponse>, tonic::Status> {
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
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/DisableToken");
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// DeleteToken removes a token.
        pub async fn delete_token(
            &mut self,
            request: impl tonic::IntoRequest<super::DeleteTokenRequest>,
        ) -> Result<tonic::Response<super::DeleteTokenResponse>, tonic::Status> {
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
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/DeleteToken");
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
        /// StartRun executes a single run of a particular pipeline.
        pub async fn start_run(
            &mut self,
            request: impl tonic::IntoRequest<super::StartRunRequest>,
        ) -> Result<tonic::Response<super::StartRunResponse>, tonic::Status> {
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
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/StartRun");
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
        /// GetTrigger returns details about a specific trigger.
        pub async fn get_trigger(
            &mut self,
            request: impl tonic::IntoRequest<super::GetTriggerRequest>,
        ) -> Result<tonic::Response<super::GetTriggerResponse>, tonic::Status> {
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
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/GetTrigger");
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// ListTriggers lists all triggers currently registered within gofer.
        pub async fn list_triggers(
            &mut self,
            request: impl tonic::IntoRequest<super::ListTriggersRequest>,
        ) -> Result<tonic::Response<super::ListTriggersResponse>, tonic::Status> {
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
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/ListTriggers");
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// GetTriggerInstalInstructions retrieves install instructions for a
        /// particular trigger.
        pub async fn get_trigger_install_instructions(
            &mut self,
            request: impl tonic::IntoRequest<super::GetTriggerInstallInstructionsRequest>,
        ) -> Result<
            tonic::Response<super::GetTriggerInstallInstructionsResponse>,
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
                "/proto.Gofer/GetTriggerInstallInstructions",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// InstallTrigger attempts to install a new trigger.
        pub async fn install_trigger(
            &mut self,
            request: impl tonic::IntoRequest<super::InstallTriggerRequest>,
        ) -> Result<tonic::Response<super::InstallTriggerResponse>, tonic::Status> {
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
                "/proto.Gofer/InstallTrigger",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// UninstallTrigger attempts to uninstall a trigger.
        pub async fn uninstall_trigger(
            &mut self,
            request: impl tonic::IntoRequest<super::UninstallTriggerRequest>,
        ) -> Result<tonic::Response<super::UninstallTriggerResponse>, tonic::Status> {
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
                "/proto.Gofer/UninstallTrigger",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// EnableTrigger attempts to enable a new trigger.
        pub async fn enable_trigger(
            &mut self,
            request: impl tonic::IntoRequest<super::EnableTriggerRequest>,
        ) -> Result<tonic::Response<super::EnableTriggerResponse>, tonic::Status> {
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
                "/proto.Gofer/EnableTrigger",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// DisableTrigger attempts to disable a new trigger.
        pub async fn disable_trigger(
            &mut self,
            request: impl tonic::IntoRequest<super::DisableTriggerRequest>,
        ) -> Result<tonic::Response<super::DisableTriggerResponse>, tonic::Status> {
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
                "/proto.Gofer/DisableTrigger",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// GetCommonTask returns details about a specific commontask.
        pub async fn get_common_task(
            &mut self,
            request: impl tonic::IntoRequest<super::GetCommonTaskRequest>,
        ) -> Result<tonic::Response<super::GetCommonTaskResponse>, tonic::Status> {
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
                "/proto.Gofer/GetCommonTask",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// ListCommonTasks lists all common tasks currently registered within gofer.
        pub async fn list_common_tasks(
            &mut self,
            request: impl tonic::IntoRequest<super::ListCommonTasksRequest>,
        ) -> Result<tonic::Response<super::ListCommonTasksResponse>, tonic::Status> {
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
                "/proto.Gofer/ListCommonTasks",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// GetCommonTaskInstalInstructions retrieves install instructions for a
        /// particular common task.
        pub async fn get_common_task_install_instructions(
            &mut self,
            request: impl tonic::IntoRequest<
                super::GetCommonTaskInstallInstructionsRequest,
            >,
        ) -> Result<
            tonic::Response<super::GetCommonTaskInstallInstructionsResponse>,
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
                "/proto.Gofer/GetCommonTaskInstallInstructions",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// InstallCommonTask attempts to install a new common task.
        pub async fn install_common_task(
            &mut self,
            request: impl tonic::IntoRequest<super::InstallCommonTaskRequest>,
        ) -> Result<tonic::Response<super::InstallCommonTaskResponse>, tonic::Status> {
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
                "/proto.Gofer/InstallCommonTask",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// UninstallCommonTask attempts to uninstall a common task.
        pub async fn uninstall_common_task(
            &mut self,
            request: impl tonic::IntoRequest<super::UninstallCommonTaskRequest>,
        ) -> Result<tonic::Response<super::UninstallCommonTaskResponse>, tonic::Status> {
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
                "/proto.Gofer/UninstallCommonTask",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// EnableCommonTask attempts to enable a new common task.
        pub async fn enable_common_task(
            &mut self,
            request: impl tonic::IntoRequest<super::EnableCommonTaskRequest>,
        ) -> Result<tonic::Response<super::EnableCommonTaskResponse>, tonic::Status> {
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
                "/proto.Gofer/EnableCommonTask",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// DisableCommonTask attempts to disable a new common task.
        pub async fn disable_common_task(
            &mut self,
            request: impl tonic::IntoRequest<super::DisableCommonTaskRequest>,
        ) -> Result<tonic::Response<super::DisableCommonTaskResponse>, tonic::Status> {
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
                "/proto.Gofer/DisableCommonTask",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// ListPipelineObjects returns a list of all pipeline object keys.
        pub async fn list_pipeline_objects(
            &mut self,
            request: impl tonic::IntoRequest<super::ListPipelineObjectsRequest>,
        ) -> Result<tonic::Response<super::ListPipelineObjectsResponse>, tonic::Status> {
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
                "/proto.Gofer/ListPipelineObjects",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// GetPipelineObject returns a single pipeline object by pipeline ID and key.
        pub async fn get_pipeline_object(
            &mut self,
            request: impl tonic::IntoRequest<super::GetPipelineObjectRequest>,
        ) -> Result<tonic::Response<super::GetPipelineObjectResponse>, tonic::Status> {
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
                "/proto.Gofer/GetPipelineObject",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// PutPipelineObject uploads a single pipeline object by pipeline ID and key.
        /// Objects which are put under the same key do not count towards the pipeline
        /// object limit.
        pub async fn put_pipeline_object(
            &mut self,
            request: impl tonic::IntoRequest<super::PutPipelineObjectRequest>,
        ) -> Result<tonic::Response<super::PutPipelineObjectResponse>, tonic::Status> {
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
                "/proto.Gofer/PutPipelineObject",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// DeletePipelineObject removes a single pipeline object by pipeline ID and
        /// key. Removing a pipeline object decrements the total count of the pipeline
        /// object limit.
        pub async fn delete_pipeline_object(
            &mut self,
            request: impl tonic::IntoRequest<super::DeletePipelineObjectRequest>,
        ) -> Result<
            tonic::Response<super::DeletePipelineObjectResponse>,
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
                "/proto.Gofer/DeletePipelineObject",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// ListRunObjects returns a list of all run object keys.
        pub async fn list_run_objects(
            &mut self,
            request: impl tonic::IntoRequest<super::ListRunObjectsRequest>,
        ) -> Result<tonic::Response<super::ListRunObjectsResponse>, tonic::Status> {
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
                "/proto.Gofer/ListRunObjects",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// GetRunObject returns the content of a single run object.
        pub async fn get_run_object(
            &mut self,
            request: impl tonic::IntoRequest<super::GetRunObjectRequest>,
        ) -> Result<tonic::Response<super::GetRunObjectResponse>, tonic::Status> {
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
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/GetRunObject");
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// PutRunObject uploads the context of an object by run ID and key.
        pub async fn put_run_object(
            &mut self,
            request: impl tonic::IntoRequest<super::PutRunObjectRequest>,
        ) -> Result<tonic::Response<super::PutRunObjectResponse>, tonic::Status> {
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
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/PutRunObject");
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// DeleteRunObject removes a specific run object by run ID and key.
        pub async fn delete_run_object(
            &mut self,
            request: impl tonic::IntoRequest<super::DeleteRunObjectRequest>,
        ) -> Result<tonic::Response<super::DeleteRunObjectResponse>, tonic::Status> {
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
                "/proto.Gofer/DeleteRunObject",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// GetPipelineSecret returns a single secret by pipeline ID and key.
        pub async fn get_pipeline_secret(
            &mut self,
            request: impl tonic::IntoRequest<super::GetPipelineSecretRequest>,
        ) -> Result<tonic::Response<super::GetPipelineSecretResponse>, tonic::Status> {
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
                "/proto.Gofer/GetPipelineSecret",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// ListPipelineSecrets returns a single secret by pipeline ID and key.
        pub async fn list_pipeline_secrets(
            &mut self,
            request: impl tonic::IntoRequest<super::ListPipelineSecretsRequest>,
        ) -> Result<tonic::Response<super::ListPipelineSecretsResponse>, tonic::Status> {
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
                "/proto.Gofer/ListPipelineSecrets",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// PutPipelineSecret uploads a single secret by pipeline ID and key.
        pub async fn put_pipeline_secret(
            &mut self,
            request: impl tonic::IntoRequest<super::PutPipelineSecretRequest>,
        ) -> Result<tonic::Response<super::PutPipelineSecretResponse>, tonic::Status> {
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
                "/proto.Gofer/PutPipelineSecret",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// DeletePipelineSecret removes a single secret by pipeline ID and
        /// key.
        pub async fn delete_pipeline_secret(
            &mut self,
            request: impl tonic::IntoRequest<super::DeletePipelineSecretRequest>,
        ) -> Result<
            tonic::Response<super::DeletePipelineSecretResponse>,
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
                "/proto.Gofer/DeletePipelineSecret",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// GetGlobalSecret returns a single secret by  key.
        pub async fn get_global_secret(
            &mut self,
            request: impl tonic::IntoRequest<super::GetGlobalSecretRequest>,
        ) -> Result<tonic::Response<super::GetGlobalSecretResponse>, tonic::Status> {
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
                "/proto.Gofer/GetGlobalSecret",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// ListGlobalSecrets returns a single secret by global ID and key.
        pub async fn list_global_secrets(
            &mut self,
            request: impl tonic::IntoRequest<super::ListGlobalSecretsRequest>,
        ) -> Result<tonic::Response<super::ListGlobalSecretsResponse>, tonic::Status> {
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
                "/proto.Gofer/ListGlobalSecrets",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// PutGlobalSecret uploads a single secret by key.
        pub async fn put_global_secret(
            &mut self,
            request: impl tonic::IntoRequest<super::PutGlobalSecretRequest>,
        ) -> Result<tonic::Response<super::PutGlobalSecretResponse>, tonic::Status> {
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
                "/proto.Gofer/PutGlobalSecret",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// DeleteGlobalSecret removes a single secret by key.
        pub async fn delete_global_secret(
            &mut self,
            request: impl tonic::IntoRequest<super::DeleteGlobalSecretRequest>,
        ) -> Result<tonic::Response<super::DeleteGlobalSecretResponse>, tonic::Status> {
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
                "/proto.Gofer/DeleteGlobalSecret",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// GetEvent returns the details of a single event.
        pub async fn get_event(
            &mut self,
            request: impl tonic::IntoRequest<super::GetEventRequest>,
        ) -> Result<tonic::Response<super::GetEventResponse>, tonic::Status> {
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
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/GetEvent");
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// ListEvents returns a streaming list of all events, ordered by
        /// oldest to newest.
        pub async fn list_events(
            &mut self,
            request: impl tonic::IntoRequest<super::ListEventsRequest>,
        ) -> Result<
            tonic::Response<tonic::codec::Streaming<super::ListEventsResponse>>,
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
            let path = http::uri::PathAndQuery::from_static("/proto.Gofer/ListEvents");
            self.inner.server_streaming(request.into_request(), path, codec).await
        }
    }
}
/// Generated client implementations.
pub mod trigger_service_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    #[derive(Debug, Clone)]
    pub struct TriggerServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl TriggerServiceClient<tonic::transport::Channel> {
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
    impl<T> TriggerServiceClient<T>
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
        ) -> TriggerServiceClient<InterceptedService<T, F>>
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
            TriggerServiceClient::new(InterceptedService::new(inner, interceptor))
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
        /// Watch blocks until the trigger has a pipeline that should be run, then it
        /// returns.
        pub async fn watch(
            &mut self,
            request: impl tonic::IntoRequest<super::TriggerWatchRequest>,
        ) -> Result<tonic::Response<super::TriggerWatchResponse>, tonic::Status> {
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
                "/proto.TriggerService/Watch",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// Info returns information on the specific plugin
        pub async fn info(
            &mut self,
            request: impl tonic::IntoRequest<super::TriggerInfoRequest>,
        ) -> Result<tonic::Response<super::TriggerInfoResponse>, tonic::Status> {
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
                "/proto.TriggerService/Info",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// Subscribe allows a trigger to keep track of all pipelines currently
        /// dependant on that trigger so that we can trigger them at appropriate times.
        pub async fn subscribe(
            &mut self,
            request: impl tonic::IntoRequest<super::TriggerSubscribeRequest>,
        ) -> Result<tonic::Response<super::TriggerSubscribeResponse>, tonic::Status> {
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
                "/proto.TriggerService/Subscribe",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// Unsubscribe allows pipelines to remove their trigger subscriptions. This is
        /// useful if the pipeline no longer needs to be notified about a specific
        /// trigger automation.
        pub async fn unsubscribe(
            &mut self,
            request: impl tonic::IntoRequest<super::TriggerUnsubscribeRequest>,
        ) -> Result<tonic::Response<super::TriggerUnsubscribeResponse>, tonic::Status> {
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
                "/proto.TriggerService/Unsubscribe",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// Shutdown tells the trigger to cleanup and gracefully shutdown. If a trigger
        /// does not shutdown in a time defined by the gofer API the trigger will
        /// instead be Force shutdown(SIGKILL). This is to say that all triggers should
        /// lean toward quick cleanups and shutdowns.
        pub async fn shutdown(
            &mut self,
            request: impl tonic::IntoRequest<super::TriggerShutdownRequest>,
        ) -> Result<tonic::Response<super::TriggerShutdownResponse>, tonic::Status> {
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
                "/proto.TriggerService/Shutdown",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// ExternalEvent are json blobs of gofer's /events endpoint. Normally
        /// webhooks.
        pub async fn external_event(
            &mut self,
            request: impl tonic::IntoRequest<super::TriggerExternalEventRequest>,
        ) -> Result<
            tonic::Response<super::TriggerExternalEventResponse>,
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
                "/proto.TriggerService/ExternalEvent",
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
        #[doc = "//////////// System RPCs //////////////"]
        ///
        /// Service RPCs exist to help with management of the Gofer service. They
        /// usually perform admin type interactions with the service as a whole and
        /// provide ways for admins to quickly repair Gofer broken states without
        /// having to stop the entire service.
        async fn get_system_info(
            &self,
            request: tonic::Request<super::GetSystemInfoRequest>,
        ) -> Result<tonic::Response<super::GetSystemInfoResponse>, tonic::Status>;
        /// RepairOrphan is used when a single run has gotten into a state that does
        /// not reflect what actually happened to the run. This can happen if the Gofer
        /// service crashes for unforeseen reasons. Usually this route is not needed as
        /// Gofer will make an attempt to resolve all orphaned runs upon startup. But
        /// in the rare case that a run gets into a bad state during the service's
        /// normal execution this route can be used to attempt to repair the orphaned
        /// run or at the very least mark it as failed so it isn't stuck in a
        /// unfinished state.
        async fn repair_orphan(
            &self,
            request: tonic::Request<super::RepairOrphanRequest>,
        ) -> Result<tonic::Response<super::RepairOrphanResponse>, tonic::Status>;
        /// ToggleEventIngress allows the admin to start or stop the execution of all
        /// pipelines within Gofer. This can be useful under some security implications
        /// or for the purposes of defining general downtime and service maintenance.
        async fn toggle_event_ingress(
            &self,
            request: tonic::Request<super::ToggleEventIngressRequest>,
        ) -> Result<tonic::Response<super::ToggleEventIngressResponse>, tonic::Status>;
        /// CreateToken manifests a new API token; This token can be a management
        /// token(the equivalent of root in Linux) or a client token. Management tokens
        /// are the only tokens that can generate tokens.
        /// Client tokens are used to manage which namespaces users have access to.
        async fn create_token(
            &self,
            request: tonic::Request<super::CreateTokenRequest>,
        ) -> Result<tonic::Response<super::CreateTokenResponse>, tonic::Status>;
        /// BootstrapToken creates the initial management token used to create all
        /// other tokens.
        async fn bootstrap_token(
            &self,
            request: tonic::Request<super::BootstrapTokenRequest>,
        ) -> Result<tonic::Response<super::BootstrapTokenResponse>, tonic::Status>;
        /// ListTokens returns information about all tokens for a particular namespace;
        async fn list_tokens(
            &self,
            request: tonic::Request<super::ListTokensRequest>,
        ) -> Result<tonic::Response<super::ListTokensResponse>, tonic::Status>;
        /// GetToken returns information about a particular token;
        async fn get_token(
            &self,
            request: tonic::Request<super::GetTokenRequest>,
        ) -> Result<tonic::Response<super::GetTokenResponse>, tonic::Status>;
        /// EnableToken makes a token usable.
        async fn enable_token(
            &self,
            request: tonic::Request<super::EnableTokenRequest>,
        ) -> Result<tonic::Response<super::EnableTokenResponse>, tonic::Status>;
        /// DisableToken makes a token unusable.
        async fn disable_token(
            &self,
            request: tonic::Request<super::DisableTokenRequest>,
        ) -> Result<tonic::Response<super::DisableTokenResponse>, tonic::Status>;
        /// DeleteToken removes a token.
        async fn delete_token(
            &self,
            request: tonic::Request<super::DeleteTokenRequest>,
        ) -> Result<tonic::Response<super::DeleteTokenResponse>, tonic::Status>;
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
        /// ListRuns returns a list of all runs by Pipeline ID. Pagination can be
        /// controlled via the offset and limit parameters of the request.
        async fn list_runs(
            &self,
            request: tonic::Request<super::ListRunsRequest>,
        ) -> Result<tonic::Response<super::ListRunsResponse>, tonic::Status>;
        /// StartRun executes a single run of a particular pipeline.
        async fn start_run(
            &self,
            request: tonic::Request<super::StartRunRequest>,
        ) -> Result<tonic::Response<super::StartRunResponse>, tonic::Status>;
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
        /// GetTrigger returns details about a specific trigger.
        async fn get_trigger(
            &self,
            request: tonic::Request<super::GetTriggerRequest>,
        ) -> Result<tonic::Response<super::GetTriggerResponse>, tonic::Status>;
        /// ListTriggers lists all triggers currently registered within gofer.
        async fn list_triggers(
            &self,
            request: tonic::Request<super::ListTriggersRequest>,
        ) -> Result<tonic::Response<super::ListTriggersResponse>, tonic::Status>;
        /// GetTriggerInstalInstructions retrieves install instructions for a
        /// particular trigger.
        async fn get_trigger_install_instructions(
            &self,
            request: tonic::Request<super::GetTriggerInstallInstructionsRequest>,
        ) -> Result<
            tonic::Response<super::GetTriggerInstallInstructionsResponse>,
            tonic::Status,
        >;
        /// InstallTrigger attempts to install a new trigger.
        async fn install_trigger(
            &self,
            request: tonic::Request<super::InstallTriggerRequest>,
        ) -> Result<tonic::Response<super::InstallTriggerResponse>, tonic::Status>;
        /// UninstallTrigger attempts to uninstall a trigger.
        async fn uninstall_trigger(
            &self,
            request: tonic::Request<super::UninstallTriggerRequest>,
        ) -> Result<tonic::Response<super::UninstallTriggerResponse>, tonic::Status>;
        /// EnableTrigger attempts to enable a new trigger.
        async fn enable_trigger(
            &self,
            request: tonic::Request<super::EnableTriggerRequest>,
        ) -> Result<tonic::Response<super::EnableTriggerResponse>, tonic::Status>;
        /// DisableTrigger attempts to disable a new trigger.
        async fn disable_trigger(
            &self,
            request: tonic::Request<super::DisableTriggerRequest>,
        ) -> Result<tonic::Response<super::DisableTriggerResponse>, tonic::Status>;
        /// GetCommonTask returns details about a specific commontask.
        async fn get_common_task(
            &self,
            request: tonic::Request<super::GetCommonTaskRequest>,
        ) -> Result<tonic::Response<super::GetCommonTaskResponse>, tonic::Status>;
        /// ListCommonTasks lists all common tasks currently registered within gofer.
        async fn list_common_tasks(
            &self,
            request: tonic::Request<super::ListCommonTasksRequest>,
        ) -> Result<tonic::Response<super::ListCommonTasksResponse>, tonic::Status>;
        /// GetCommonTaskInstalInstructions retrieves install instructions for a
        /// particular common task.
        async fn get_common_task_install_instructions(
            &self,
            request: tonic::Request<super::GetCommonTaskInstallInstructionsRequest>,
        ) -> Result<
            tonic::Response<super::GetCommonTaskInstallInstructionsResponse>,
            tonic::Status,
        >;
        /// InstallCommonTask attempts to install a new common task.
        async fn install_common_task(
            &self,
            request: tonic::Request<super::InstallCommonTaskRequest>,
        ) -> Result<tonic::Response<super::InstallCommonTaskResponse>, tonic::Status>;
        /// UninstallCommonTask attempts to uninstall a common task.
        async fn uninstall_common_task(
            &self,
            request: tonic::Request<super::UninstallCommonTaskRequest>,
        ) -> Result<tonic::Response<super::UninstallCommonTaskResponse>, tonic::Status>;
        /// EnableCommonTask attempts to enable a new common task.
        async fn enable_common_task(
            &self,
            request: tonic::Request<super::EnableCommonTaskRequest>,
        ) -> Result<tonic::Response<super::EnableCommonTaskResponse>, tonic::Status>;
        /// DisableCommonTask attempts to disable a new common task.
        async fn disable_common_task(
            &self,
            request: tonic::Request<super::DisableCommonTaskRequest>,
        ) -> Result<tonic::Response<super::DisableCommonTaskResponse>, tonic::Status>;
        /// ListPipelineObjects returns a list of all pipeline object keys.
        async fn list_pipeline_objects(
            &self,
            request: tonic::Request<super::ListPipelineObjectsRequest>,
        ) -> Result<tonic::Response<super::ListPipelineObjectsResponse>, tonic::Status>;
        /// GetPipelineObject returns a single pipeline object by pipeline ID and key.
        async fn get_pipeline_object(
            &self,
            request: tonic::Request<super::GetPipelineObjectRequest>,
        ) -> Result<tonic::Response<super::GetPipelineObjectResponse>, tonic::Status>;
        /// PutPipelineObject uploads a single pipeline object by pipeline ID and key.
        /// Objects which are put under the same key do not count towards the pipeline
        /// object limit.
        async fn put_pipeline_object(
            &self,
            request: tonic::Request<super::PutPipelineObjectRequest>,
        ) -> Result<tonic::Response<super::PutPipelineObjectResponse>, tonic::Status>;
        /// DeletePipelineObject removes a single pipeline object by pipeline ID and
        /// key. Removing a pipeline object decrements the total count of the pipeline
        /// object limit.
        async fn delete_pipeline_object(
            &self,
            request: tonic::Request<super::DeletePipelineObjectRequest>,
        ) -> Result<tonic::Response<super::DeletePipelineObjectResponse>, tonic::Status>;
        /// ListRunObjects returns a list of all run object keys.
        async fn list_run_objects(
            &self,
            request: tonic::Request<super::ListRunObjectsRequest>,
        ) -> Result<tonic::Response<super::ListRunObjectsResponse>, tonic::Status>;
        /// GetRunObject returns the content of a single run object.
        async fn get_run_object(
            &self,
            request: tonic::Request<super::GetRunObjectRequest>,
        ) -> Result<tonic::Response<super::GetRunObjectResponse>, tonic::Status>;
        /// PutRunObject uploads the context of an object by run ID and key.
        async fn put_run_object(
            &self,
            request: tonic::Request<super::PutRunObjectRequest>,
        ) -> Result<tonic::Response<super::PutRunObjectResponse>, tonic::Status>;
        /// DeleteRunObject removes a specific run object by run ID and key.
        async fn delete_run_object(
            &self,
            request: tonic::Request<super::DeleteRunObjectRequest>,
        ) -> Result<tonic::Response<super::DeleteRunObjectResponse>, tonic::Status>;
        /// GetPipelineSecret returns a single secret by pipeline ID and key.
        async fn get_pipeline_secret(
            &self,
            request: tonic::Request<super::GetPipelineSecretRequest>,
        ) -> Result<tonic::Response<super::GetPipelineSecretResponse>, tonic::Status>;
        /// ListPipelineSecrets returns a single secret by pipeline ID and key.
        async fn list_pipeline_secrets(
            &self,
            request: tonic::Request<super::ListPipelineSecretsRequest>,
        ) -> Result<tonic::Response<super::ListPipelineSecretsResponse>, tonic::Status>;
        /// PutPipelineSecret uploads a single secret by pipeline ID and key.
        async fn put_pipeline_secret(
            &self,
            request: tonic::Request<super::PutPipelineSecretRequest>,
        ) -> Result<tonic::Response<super::PutPipelineSecretResponse>, tonic::Status>;
        /// DeletePipelineSecret removes a single secret by pipeline ID and
        /// key.
        async fn delete_pipeline_secret(
            &self,
            request: tonic::Request<super::DeletePipelineSecretRequest>,
        ) -> Result<tonic::Response<super::DeletePipelineSecretResponse>, tonic::Status>;
        /// GetGlobalSecret returns a single secret by  key.
        async fn get_global_secret(
            &self,
            request: tonic::Request<super::GetGlobalSecretRequest>,
        ) -> Result<tonic::Response<super::GetGlobalSecretResponse>, tonic::Status>;
        /// ListGlobalSecrets returns a single secret by global ID and key.
        async fn list_global_secrets(
            &self,
            request: tonic::Request<super::ListGlobalSecretsRequest>,
        ) -> Result<tonic::Response<super::ListGlobalSecretsResponse>, tonic::Status>;
        /// PutGlobalSecret uploads a single secret by key.
        async fn put_global_secret(
            &self,
            request: tonic::Request<super::PutGlobalSecretRequest>,
        ) -> Result<tonic::Response<super::PutGlobalSecretResponse>, tonic::Status>;
        /// DeleteGlobalSecret removes a single secret by key.
        async fn delete_global_secret(
            &self,
            request: tonic::Request<super::DeleteGlobalSecretRequest>,
        ) -> Result<tonic::Response<super::DeleteGlobalSecretResponse>, tonic::Status>;
        /// GetEvent returns the details of a single event.
        async fn get_event(
            &self,
            request: tonic::Request<super::GetEventRequest>,
        ) -> Result<tonic::Response<super::GetEventResponse>, tonic::Status>;
        ///Server streaming response type for the ListEvents method.
        type ListEventsStream: futures_core::Stream<
                Item = Result<super::ListEventsResponse, tonic::Status>,
            >
            + Send
            + 'static;
        /// ListEvents returns a streaming list of all events, ordered by
        /// oldest to newest.
        async fn list_events(
            &self,
            request: tonic::Request<super::ListEventsRequest>,
        ) -> Result<tonic::Response<Self::ListEventsStream>, tonic::Status>;
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
                "/proto.Gofer/RepairOrphan" => {
                    #[allow(non_camel_case_types)]
                    struct RepairOrphanSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::RepairOrphanRequest>
                    for RepairOrphanSvc<T> {
                        type Response = super::RepairOrphanResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::RepairOrphanRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).repair_orphan(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = RepairOrphanSvc(inner);
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
                "/proto.Gofer/ToggleEventIngress" => {
                    #[allow(non_camel_case_types)]
                    struct ToggleEventIngressSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::ToggleEventIngressRequest>
                    for ToggleEventIngressSvc<T> {
                        type Response = super::ToggleEventIngressResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ToggleEventIngressRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).toggle_event_ingress(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ToggleEventIngressSvc(inner);
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
                "/proto.Gofer/CreateToken" => {
                    #[allow(non_camel_case_types)]
                    struct CreateTokenSvc<T: Gofer>(pub Arc<T>);
                    impl<T: Gofer> tonic::server::UnaryService<super::CreateTokenRequest>
                    for CreateTokenSvc<T> {
                        type Response = super::CreateTokenResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::CreateTokenRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).create_token(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = CreateTokenSvc(inner);
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
                "/proto.Gofer/BootstrapToken" => {
                    #[allow(non_camel_case_types)]
                    struct BootstrapTokenSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::BootstrapTokenRequest>
                    for BootstrapTokenSvc<T> {
                        type Response = super::BootstrapTokenResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::BootstrapTokenRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).bootstrap_token(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = BootstrapTokenSvc(inner);
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
                "/proto.Gofer/ListTokens" => {
                    #[allow(non_camel_case_types)]
                    struct ListTokensSvc<T: Gofer>(pub Arc<T>);
                    impl<T: Gofer> tonic::server::UnaryService<super::ListTokensRequest>
                    for ListTokensSvc<T> {
                        type Response = super::ListTokensResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ListTokensRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).list_tokens(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ListTokensSvc(inner);
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
                "/proto.Gofer/GetToken" => {
                    #[allow(non_camel_case_types)]
                    struct GetTokenSvc<T: Gofer>(pub Arc<T>);
                    impl<T: Gofer> tonic::server::UnaryService<super::GetTokenRequest>
                    for GetTokenSvc<T> {
                        type Response = super::GetTokenResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetTokenRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).get_token(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetTokenSvc(inner);
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
                "/proto.Gofer/EnableToken" => {
                    #[allow(non_camel_case_types)]
                    struct EnableTokenSvc<T: Gofer>(pub Arc<T>);
                    impl<T: Gofer> tonic::server::UnaryService<super::EnableTokenRequest>
                    for EnableTokenSvc<T> {
                        type Response = super::EnableTokenResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::EnableTokenRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).enable_token(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = EnableTokenSvc(inner);
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
                "/proto.Gofer/DisableToken" => {
                    #[allow(non_camel_case_types)]
                    struct DisableTokenSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::DisableTokenRequest>
                    for DisableTokenSvc<T> {
                        type Response = super::DisableTokenResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::DisableTokenRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).disable_token(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = DisableTokenSvc(inner);
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
                "/proto.Gofer/DeleteToken" => {
                    #[allow(non_camel_case_types)]
                    struct DeleteTokenSvc<T: Gofer>(pub Arc<T>);
                    impl<T: Gofer> tonic::server::UnaryService<super::DeleteTokenRequest>
                    for DeleteTokenSvc<T> {
                        type Response = super::DeleteTokenResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::DeleteTokenRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).delete_token(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = DeleteTokenSvc(inner);
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
                "/proto.Gofer/StartRun" => {
                    #[allow(non_camel_case_types)]
                    struct StartRunSvc<T: Gofer>(pub Arc<T>);
                    impl<T: Gofer> tonic::server::UnaryService<super::StartRunRequest>
                    for StartRunSvc<T> {
                        type Response = super::StartRunResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::StartRunRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).start_run(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = StartRunSvc(inner);
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
                "/proto.Gofer/GetTrigger" => {
                    #[allow(non_camel_case_types)]
                    struct GetTriggerSvc<T: Gofer>(pub Arc<T>);
                    impl<T: Gofer> tonic::server::UnaryService<super::GetTriggerRequest>
                    for GetTriggerSvc<T> {
                        type Response = super::GetTriggerResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetTriggerRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).get_trigger(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetTriggerSvc(inner);
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
                "/proto.Gofer/ListTriggers" => {
                    #[allow(non_camel_case_types)]
                    struct ListTriggersSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::ListTriggersRequest>
                    for ListTriggersSvc<T> {
                        type Response = super::ListTriggersResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ListTriggersRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).list_triggers(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ListTriggersSvc(inner);
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
                "/proto.Gofer/GetTriggerInstallInstructions" => {
                    #[allow(non_camel_case_types)]
                    struct GetTriggerInstallInstructionsSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<
                        super::GetTriggerInstallInstructionsRequest,
                    > for GetTriggerInstallInstructionsSvc<T> {
                        type Response = super::GetTriggerInstallInstructionsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::GetTriggerInstallInstructionsRequest,
                            >,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).get_trigger_install_instructions(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetTriggerInstallInstructionsSvc(inner);
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
                "/proto.Gofer/InstallTrigger" => {
                    #[allow(non_camel_case_types)]
                    struct InstallTriggerSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::InstallTriggerRequest>
                    for InstallTriggerSvc<T> {
                        type Response = super::InstallTriggerResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::InstallTriggerRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).install_trigger(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = InstallTriggerSvc(inner);
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
                "/proto.Gofer/UninstallTrigger" => {
                    #[allow(non_camel_case_types)]
                    struct UninstallTriggerSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::UninstallTriggerRequest>
                    for UninstallTriggerSvc<T> {
                        type Response = super::UninstallTriggerResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::UninstallTriggerRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).uninstall_trigger(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = UninstallTriggerSvc(inner);
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
                "/proto.Gofer/EnableTrigger" => {
                    #[allow(non_camel_case_types)]
                    struct EnableTriggerSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::EnableTriggerRequest>
                    for EnableTriggerSvc<T> {
                        type Response = super::EnableTriggerResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::EnableTriggerRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).enable_trigger(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = EnableTriggerSvc(inner);
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
                "/proto.Gofer/DisableTrigger" => {
                    #[allow(non_camel_case_types)]
                    struct DisableTriggerSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::DisableTriggerRequest>
                    for DisableTriggerSvc<T> {
                        type Response = super::DisableTriggerResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::DisableTriggerRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).disable_trigger(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = DisableTriggerSvc(inner);
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
                "/proto.Gofer/GetCommonTask" => {
                    #[allow(non_camel_case_types)]
                    struct GetCommonTaskSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::GetCommonTaskRequest>
                    for GetCommonTaskSvc<T> {
                        type Response = super::GetCommonTaskResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetCommonTaskRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).get_common_task(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetCommonTaskSvc(inner);
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
                "/proto.Gofer/ListCommonTasks" => {
                    #[allow(non_camel_case_types)]
                    struct ListCommonTasksSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::ListCommonTasksRequest>
                    for ListCommonTasksSvc<T> {
                        type Response = super::ListCommonTasksResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ListCommonTasksRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).list_common_tasks(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ListCommonTasksSvc(inner);
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
                "/proto.Gofer/GetCommonTaskInstallInstructions" => {
                    #[allow(non_camel_case_types)]
                    struct GetCommonTaskInstallInstructionsSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<
                        super::GetCommonTaskInstallInstructionsRequest,
                    > for GetCommonTaskInstallInstructionsSvc<T> {
                        type Response = super::GetCommonTaskInstallInstructionsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::GetCommonTaskInstallInstructionsRequest,
                            >,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).get_common_task_install_instructions(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetCommonTaskInstallInstructionsSvc(inner);
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
                "/proto.Gofer/InstallCommonTask" => {
                    #[allow(non_camel_case_types)]
                    struct InstallCommonTaskSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::InstallCommonTaskRequest>
                    for InstallCommonTaskSvc<T> {
                        type Response = super::InstallCommonTaskResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::InstallCommonTaskRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).install_common_task(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = InstallCommonTaskSvc(inner);
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
                "/proto.Gofer/UninstallCommonTask" => {
                    #[allow(non_camel_case_types)]
                    struct UninstallCommonTaskSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::UninstallCommonTaskRequest>
                    for UninstallCommonTaskSvc<T> {
                        type Response = super::UninstallCommonTaskResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::UninstallCommonTaskRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).uninstall_common_task(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = UninstallCommonTaskSvc(inner);
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
                "/proto.Gofer/EnableCommonTask" => {
                    #[allow(non_camel_case_types)]
                    struct EnableCommonTaskSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::EnableCommonTaskRequest>
                    for EnableCommonTaskSvc<T> {
                        type Response = super::EnableCommonTaskResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::EnableCommonTaskRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).enable_common_task(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = EnableCommonTaskSvc(inner);
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
                "/proto.Gofer/DisableCommonTask" => {
                    #[allow(non_camel_case_types)]
                    struct DisableCommonTaskSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::DisableCommonTaskRequest>
                    for DisableCommonTaskSvc<T> {
                        type Response = super::DisableCommonTaskResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::DisableCommonTaskRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).disable_common_task(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = DisableCommonTaskSvc(inner);
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
                "/proto.Gofer/ListPipelineObjects" => {
                    #[allow(non_camel_case_types)]
                    struct ListPipelineObjectsSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::ListPipelineObjectsRequest>
                    for ListPipelineObjectsSvc<T> {
                        type Response = super::ListPipelineObjectsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ListPipelineObjectsRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).list_pipeline_objects(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ListPipelineObjectsSvc(inner);
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
                "/proto.Gofer/GetPipelineObject" => {
                    #[allow(non_camel_case_types)]
                    struct GetPipelineObjectSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::GetPipelineObjectRequest>
                    for GetPipelineObjectSvc<T> {
                        type Response = super::GetPipelineObjectResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetPipelineObjectRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).get_pipeline_object(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetPipelineObjectSvc(inner);
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
                "/proto.Gofer/PutPipelineObject" => {
                    #[allow(non_camel_case_types)]
                    struct PutPipelineObjectSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::PutPipelineObjectRequest>
                    for PutPipelineObjectSvc<T> {
                        type Response = super::PutPipelineObjectResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::PutPipelineObjectRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).put_pipeline_object(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = PutPipelineObjectSvc(inner);
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
                "/proto.Gofer/DeletePipelineObject" => {
                    #[allow(non_camel_case_types)]
                    struct DeletePipelineObjectSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::DeletePipelineObjectRequest>
                    for DeletePipelineObjectSvc<T> {
                        type Response = super::DeletePipelineObjectResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::DeletePipelineObjectRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).delete_pipeline_object(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = DeletePipelineObjectSvc(inner);
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
                "/proto.Gofer/ListRunObjects" => {
                    #[allow(non_camel_case_types)]
                    struct ListRunObjectsSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::ListRunObjectsRequest>
                    for ListRunObjectsSvc<T> {
                        type Response = super::ListRunObjectsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ListRunObjectsRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).list_run_objects(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ListRunObjectsSvc(inner);
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
                "/proto.Gofer/GetRunObject" => {
                    #[allow(non_camel_case_types)]
                    struct GetRunObjectSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::GetRunObjectRequest>
                    for GetRunObjectSvc<T> {
                        type Response = super::GetRunObjectResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetRunObjectRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).get_run_object(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetRunObjectSvc(inner);
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
                "/proto.Gofer/PutRunObject" => {
                    #[allow(non_camel_case_types)]
                    struct PutRunObjectSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::PutRunObjectRequest>
                    for PutRunObjectSvc<T> {
                        type Response = super::PutRunObjectResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::PutRunObjectRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).put_run_object(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = PutRunObjectSvc(inner);
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
                "/proto.Gofer/DeleteRunObject" => {
                    #[allow(non_camel_case_types)]
                    struct DeleteRunObjectSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::DeleteRunObjectRequest>
                    for DeleteRunObjectSvc<T> {
                        type Response = super::DeleteRunObjectResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::DeleteRunObjectRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).delete_run_object(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = DeleteRunObjectSvc(inner);
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
                "/proto.Gofer/GetPipelineSecret" => {
                    #[allow(non_camel_case_types)]
                    struct GetPipelineSecretSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::GetPipelineSecretRequest>
                    for GetPipelineSecretSvc<T> {
                        type Response = super::GetPipelineSecretResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetPipelineSecretRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).get_pipeline_secret(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetPipelineSecretSvc(inner);
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
                "/proto.Gofer/ListPipelineSecrets" => {
                    #[allow(non_camel_case_types)]
                    struct ListPipelineSecretsSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::ListPipelineSecretsRequest>
                    for ListPipelineSecretsSvc<T> {
                        type Response = super::ListPipelineSecretsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ListPipelineSecretsRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).list_pipeline_secrets(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ListPipelineSecretsSvc(inner);
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
                "/proto.Gofer/PutPipelineSecret" => {
                    #[allow(non_camel_case_types)]
                    struct PutPipelineSecretSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::PutPipelineSecretRequest>
                    for PutPipelineSecretSvc<T> {
                        type Response = super::PutPipelineSecretResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::PutPipelineSecretRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).put_pipeline_secret(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = PutPipelineSecretSvc(inner);
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
                "/proto.Gofer/DeletePipelineSecret" => {
                    #[allow(non_camel_case_types)]
                    struct DeletePipelineSecretSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::DeletePipelineSecretRequest>
                    for DeletePipelineSecretSvc<T> {
                        type Response = super::DeletePipelineSecretResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::DeletePipelineSecretRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).delete_pipeline_secret(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = DeletePipelineSecretSvc(inner);
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
                "/proto.Gofer/GetGlobalSecret" => {
                    #[allow(non_camel_case_types)]
                    struct GetGlobalSecretSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::GetGlobalSecretRequest>
                    for GetGlobalSecretSvc<T> {
                        type Response = super::GetGlobalSecretResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetGlobalSecretRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).get_global_secret(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetGlobalSecretSvc(inner);
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
                "/proto.Gofer/ListGlobalSecrets" => {
                    #[allow(non_camel_case_types)]
                    struct ListGlobalSecretsSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::ListGlobalSecretsRequest>
                    for ListGlobalSecretsSvc<T> {
                        type Response = super::ListGlobalSecretsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ListGlobalSecretsRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).list_global_secrets(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ListGlobalSecretsSvc(inner);
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
                "/proto.Gofer/PutGlobalSecret" => {
                    #[allow(non_camel_case_types)]
                    struct PutGlobalSecretSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::PutGlobalSecretRequest>
                    for PutGlobalSecretSvc<T> {
                        type Response = super::PutGlobalSecretResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::PutGlobalSecretRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).put_global_secret(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = PutGlobalSecretSvc(inner);
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
                "/proto.Gofer/DeleteGlobalSecret" => {
                    #[allow(non_camel_case_types)]
                    struct DeleteGlobalSecretSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::UnaryService<super::DeleteGlobalSecretRequest>
                    for DeleteGlobalSecretSvc<T> {
                        type Response = super::DeleteGlobalSecretResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::DeleteGlobalSecretRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).delete_global_secret(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = DeleteGlobalSecretSvc(inner);
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
                "/proto.Gofer/GetEvent" => {
                    #[allow(non_camel_case_types)]
                    struct GetEventSvc<T: Gofer>(pub Arc<T>);
                    impl<T: Gofer> tonic::server::UnaryService<super::GetEventRequest>
                    for GetEventSvc<T> {
                        type Response = super::GetEventResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetEventRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).get_event(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetEventSvc(inner);
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
                "/proto.Gofer/ListEvents" => {
                    #[allow(non_camel_case_types)]
                    struct ListEventsSvc<T: Gofer>(pub Arc<T>);
                    impl<
                        T: Gofer,
                    > tonic::server::ServerStreamingService<super::ListEventsRequest>
                    for ListEventsSvc<T> {
                        type Response = super::ListEventsResponse;
                        type ResponseStream = T::ListEventsStream;
                        type Future = BoxFuture<
                            tonic::Response<Self::ResponseStream>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ListEventsRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).list_events(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ListEventsSvc(inner);
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
/// Generated server implementations.
pub mod trigger_service_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    ///Generated trait containing gRPC methods that should be implemented for use with TriggerServiceServer.
    #[async_trait]
    pub trait TriggerService: Send + Sync + 'static {
        /// Watch blocks until the trigger has a pipeline that should be run, then it
        /// returns.
        async fn watch(
            &self,
            request: tonic::Request<super::TriggerWatchRequest>,
        ) -> Result<tonic::Response<super::TriggerWatchResponse>, tonic::Status>;
        /// Info returns information on the specific plugin
        async fn info(
            &self,
            request: tonic::Request<super::TriggerInfoRequest>,
        ) -> Result<tonic::Response<super::TriggerInfoResponse>, tonic::Status>;
        /// Subscribe allows a trigger to keep track of all pipelines currently
        /// dependant on that trigger so that we can trigger them at appropriate times.
        async fn subscribe(
            &self,
            request: tonic::Request<super::TriggerSubscribeRequest>,
        ) -> Result<tonic::Response<super::TriggerSubscribeResponse>, tonic::Status>;
        /// Unsubscribe allows pipelines to remove their trigger subscriptions. This is
        /// useful if the pipeline no longer needs to be notified about a specific
        /// trigger automation.
        async fn unsubscribe(
            &self,
            request: tonic::Request<super::TriggerUnsubscribeRequest>,
        ) -> Result<tonic::Response<super::TriggerUnsubscribeResponse>, tonic::Status>;
        /// Shutdown tells the trigger to cleanup and gracefully shutdown. If a trigger
        /// does not shutdown in a time defined by the gofer API the trigger will
        /// instead be Force shutdown(SIGKILL). This is to say that all triggers should
        /// lean toward quick cleanups and shutdowns.
        async fn shutdown(
            &self,
            request: tonic::Request<super::TriggerShutdownRequest>,
        ) -> Result<tonic::Response<super::TriggerShutdownResponse>, tonic::Status>;
        /// ExternalEvent are json blobs of gofer's /events endpoint. Normally
        /// webhooks.
        async fn external_event(
            &self,
            request: tonic::Request<super::TriggerExternalEventRequest>,
        ) -> Result<tonic::Response<super::TriggerExternalEventResponse>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct TriggerServiceServer<T: TriggerService> {
        inner: _Inner<T>,
        accept_compression_encodings: (),
        send_compression_encodings: (),
    }
    struct _Inner<T>(Arc<T>);
    impl<T: TriggerService> TriggerServiceServer<T> {
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
    impl<T, B> tonic::codegen::Service<http::Request<B>> for TriggerServiceServer<T>
    where
        T: TriggerService,
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
                "/proto.TriggerService/Watch" => {
                    #[allow(non_camel_case_types)]
                    struct WatchSvc<T: TriggerService>(pub Arc<T>);
                    impl<
                        T: TriggerService,
                    > tonic::server::UnaryService<super::TriggerWatchRequest>
                    for WatchSvc<T> {
                        type Response = super::TriggerWatchResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::TriggerWatchRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).watch(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = WatchSvc(inner);
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
                "/proto.TriggerService/Info" => {
                    #[allow(non_camel_case_types)]
                    struct InfoSvc<T: TriggerService>(pub Arc<T>);
                    impl<
                        T: TriggerService,
                    > tonic::server::UnaryService<super::TriggerInfoRequest>
                    for InfoSvc<T> {
                        type Response = super::TriggerInfoResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::TriggerInfoRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).info(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = InfoSvc(inner);
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
                "/proto.TriggerService/Subscribe" => {
                    #[allow(non_camel_case_types)]
                    struct SubscribeSvc<T: TriggerService>(pub Arc<T>);
                    impl<
                        T: TriggerService,
                    > tonic::server::UnaryService<super::TriggerSubscribeRequest>
                    for SubscribeSvc<T> {
                        type Response = super::TriggerSubscribeResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::TriggerSubscribeRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).subscribe(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = SubscribeSvc(inner);
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
                "/proto.TriggerService/Unsubscribe" => {
                    #[allow(non_camel_case_types)]
                    struct UnsubscribeSvc<T: TriggerService>(pub Arc<T>);
                    impl<
                        T: TriggerService,
                    > tonic::server::UnaryService<super::TriggerUnsubscribeRequest>
                    for UnsubscribeSvc<T> {
                        type Response = super::TriggerUnsubscribeResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::TriggerUnsubscribeRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).unsubscribe(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = UnsubscribeSvc(inner);
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
                "/proto.TriggerService/Shutdown" => {
                    #[allow(non_camel_case_types)]
                    struct ShutdownSvc<T: TriggerService>(pub Arc<T>);
                    impl<
                        T: TriggerService,
                    > tonic::server::UnaryService<super::TriggerShutdownRequest>
                    for ShutdownSvc<T> {
                        type Response = super::TriggerShutdownResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::TriggerShutdownRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).shutdown(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ShutdownSvc(inner);
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
                "/proto.TriggerService/ExternalEvent" => {
                    #[allow(non_camel_case_types)]
                    struct ExternalEventSvc<T: TriggerService>(pub Arc<T>);
                    impl<
                        T: TriggerService,
                    > tonic::server::UnaryService<super::TriggerExternalEventRequest>
                    for ExternalEventSvc<T> {
                        type Response = super::TriggerExternalEventResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::TriggerExternalEventRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).external_event(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ExternalEventSvc(inner);
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
    impl<T: TriggerService> Clone for TriggerServiceServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }
    impl<T: TriggerService> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: TriggerService> tonic::transport::NamedService for TriggerServiceServer<T> {
        const NAME: &'static str = "proto.TriggerService";
    }
}
