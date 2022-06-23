use gofer_proto::{
    TriggerExternalEventRequest, TriggerExternalEventResponse, TriggerInfoRequest,
    TriggerInfoResponse, TriggerShutdownRequest, TriggerShutdownResponse, TriggerSubscribeRequest,
    TriggerSubscribeResponse, TriggerUnsubscribeRequest, TriggerUnsubscribeResponse,
    TriggerWatchRequest, TriggerWatchResponse,
};

pub enum TriggerError {
    Unknown,
    FailedPrecondition,
}

/// The Trigger interface provides a light wrapper around the GRPC trigger interface. This light wrapper
/// provides the caller with a clear interface to implement and allows this package to bake in common
/// functionality among all triggers.
trait Trigger {
    /// Blocks until the trigger has a pipeline that should be run, then it returns. This is ideal for
    /// setting the check endpoint as a channel result.
    fn watch(req: TriggerWatchRequest) -> Result<TriggerWatchResponse, TriggerError>;

    /// Returns information on the specific trigger plugin. Used as a startup health endpoint by the main
    /// Gofer process.
    fn info(req: TriggerInfoRequest) -> Result<TriggerInfoResponse, TriggerError>;

    /// Allows a trigger to keep track of all pipelines currently dependent on that trigger
    /// so that we can trigger them at appropriate times.
    fn subscribe(req: TriggerSubscribeRequest) -> Result<TriggerSubscribeResponse, TriggerError>;

    /// Allows pipelines to remove their trigger subscriptions. This is useful if the pipeline no longer
    /// needs to be notified about a specific trigger automation.
    fn unsubscribe(
        req: TriggerUnsubscribeRequest,
    ) -> Result<TriggerUnsubscribeResponse, TriggerError>;

    /// Tells the trigger to cleanup and gracefully shutdown. If a trigger does not shutdown
    /// in a tie defined by the Gofer API, it will instead be forced(SIGKILL). This is to say that all
    /// triggers should lean toward quick cleanups and shutdowns.
    fn shutdown(req: TriggerShutdownRequest) -> Result<TriggerShutdownResponse, TriggerError>;

    /// Json blobs of Gofer's external /events endpoint. Normally webhooks.
    fn external_event(
        req: TriggerExternalEventRequest,
    ) -> Result<TriggerExternalEventResponse, TriggerError>;
}
