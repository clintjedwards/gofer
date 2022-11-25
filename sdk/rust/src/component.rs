use gofer_proto::{
    ExtensionExternalEventRequest, ExtensionExternalEventResponse, ExtensionInfoRequest,
    ExtensionInfoResponse, ExtensionShutdownRequest, ExtensionShutdownResponse,
    ExtensionSubscribeRequest, ExtensionSubscribeResponse, ExtensionUnsubscribeRequest,
    ExtensionUnsubscribeResponse, ExtensionWatchRequest, ExtensionWatchResponse,
};

pub enum ExtensionError {
    Unknown,
    FailedPrecondition,
}

/// The Extension interface provides a light wrapper around the GRPC extension interface. This light wrapper
/// provides the caller with a clear interface to implement and allows this package to bake in common
/// functionality among all extensions.
trait Extension {
    /// Blocks until the extension has a pipeline that should be run, then it returns. This is ideal for
    /// setting the check endpoint as a channel result.
    fn watch(req: ExtensionWatchRequest) -> Result<ExtensionWatchResponse, ExtensionError>;

    /// Returns information on the specific extension plugin. Used as a startup health endpoint by the main
    /// Gofer process.
    fn info(req: ExtensionInfoRequest) -> Result<ExtensionInfoResponse, ExtensionError>;

    /// Allows a extension to keep track of all pipelines currently dependent on that extension
    /// so that we can extension them at appropriate times.
    fn subscribe(
        req: ExtensionSubscribeRequest,
    ) -> Result<ExtensionSubscribeResponse, ExtensionError>;

    /// Allows pipelines to remove their extension subscriptions. This is useful if the pipeline no longer
    /// needs to be notified about a specific extension automation.
    fn unsubscribe(
        req: ExtensionUnsubscribeRequest,
    ) -> Result<ExtensionUnsubscribeResponse, ExtensionError>;

    /// Tells the extension to cleanup and gracefully shutdown. If a extension does not shutdown
    /// in a tie defined by the Gofer API, it will instead be forced(SIGKILL). This is to say that all
    /// extensions should lean toward quick cleanups and shutdowns.
    fn shutdown(req: ExtensionShutdownRequest)
        -> Result<ExtensionShutdownResponse, ExtensionError>;

    /// Json blobs of Gofer's external /events endpoint. Normally webhooks.
    fn external_event(
        req: ExtensionExternalEventRequest,
    ) -> Result<ExtensionExternalEventResponse, ExtensionError>;
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InstallInstruction {
    Query { text: String, config_key: String },
    Message { text: String },
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub struct InstallInstructions {
    pub instructions: Vec<InstallInstruction>,
}

impl InstallInstructions {
    pub fn add_message(mut self, text: &str) -> Self {
        self.instructions.push(InstallInstruction::Message {
            text: text.to_string(),
        });
        self
    }

    pub fn add_query(mut self, text: &str, config_key: &str) -> Self {
        self.instructions.push(InstallInstruction::Query {
            text: text.to_string(),
            config_key: config_key.to_string(),
        });
        self
    }

    pub fn json(self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // IMPORTANT: Changing the `expected` string in this function requires you to change the golang sister function in
    // the golang sdk.
    #[test]
    fn test_instructions() {
        let output = InstallInstructions::default()
            .add_message("test")
            .add_query("test", "config")
            .json()
            .unwrap();

        let expected = r#"{"instructions":[{"message":{"text":"test"}},{"query":{"text":"test","config_key":"config"}}]}"#;

        assert_eq!(output, expected)
    }
}
