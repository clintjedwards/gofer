#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WatchRequest {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WatchResponse {
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
    #[prost(enumeration="watch_response::Result", tag="5")]
    pub result: i32,
    /// Metadata is passed to the tasks as extra environment variables.
    #[prost(map="string, string", tag="6")]
    pub metadata: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
}
/// Nested message and enum types in `WatchResponse`.
pub mod watch_response {
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
pub struct InfoRequest {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InfoResponse {
    /// kind corresponds a unique trigger identifier, this is passed as a envvar
    /// via the main process(and as such can be left empty), as the main process
    /// container the configuration for which trigger "kind" corresponds to which
    /// trigger container.
    #[prost(string, tag="1")]
    pub kind: ::prost::alloc::string::String,
    /// Triggers are allowed to provide a link to more extensive documentation on
    /// how to use and configure them.
    #[prost(string, tag="2")]
    pub documentation: ::prost::alloc::string::String,
    /// A listing of all registered pipelines in the format: <namespace>/<pipeline>
    #[prost(string, repeated, tag="3")]
    pub registered: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SubscribeRequest {
    /// unique identifier for associated namespace
    #[prost(string, tag="1")]
    pub namespace_id: ::prost::alloc::string::String,
    /// unique identifier for associated pipeline
    #[prost(string, tag="2")]
    pub pipeline_id: ::prost::alloc::string::String,
    /// pipeline specific subscription id
    #[prost(string, tag="3")]
    pub pipeline_trigger_label: ::prost::alloc::string::String,
    /// pipelines are allowed to pass a configuration to triggers denoting what
    /// specific settings they might like for a specific trigger. The acceptable
    /// values of this config map is defined by the triggers and should be
    /// mentioned in documentation.
    ///
    /// Additionally, the trigger should verify config settings and pass back an
    /// error when it does not meet requirements.
    #[prost(map="string, string", tag="4")]
    pub config: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SubscribeResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UnsubscribeRequest {
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
pub struct UnsubscribeResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ShutdownRequest {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ShutdownResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExternalEventRequest {
    #[prost(bytes="vec", tag="1")]
    pub payload: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExternalEventResponse {
}
/// Generated client implementations.
pub mod trigger_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    #[derive(Debug, Clone)]
    pub struct TriggerClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl TriggerClient<tonic::transport::Channel> {
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
    impl<T> TriggerClient<T>
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
        ) -> TriggerClient<InterceptedService<T, F>>
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
            TriggerClient::new(InterceptedService::new(inner, interceptor))
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
            request: impl tonic::IntoRequest<super::WatchRequest>,
        ) -> Result<tonic::Response<super::WatchResponse>, tonic::Status> {
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
            let path = http::uri::PathAndQuery::from_static("/sdkProto.Trigger/Watch");
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// Info returns information on the specific plugin
        pub async fn info(
            &mut self,
            request: impl tonic::IntoRequest<super::InfoRequest>,
        ) -> Result<tonic::Response<super::InfoResponse>, tonic::Status> {
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
            let path = http::uri::PathAndQuery::from_static("/sdkProto.Trigger/Info");
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// Subscribe allows a trigger to keep track of all pipelines currently
        /// dependant on that trigger so that we can trigger them at appropriate times.
        pub async fn subscribe(
            &mut self,
            request: impl tonic::IntoRequest<super::SubscribeRequest>,
        ) -> Result<tonic::Response<super::SubscribeResponse>, tonic::Status> {
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
                "/sdkProto.Trigger/Subscribe",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// Unsubscribe allows pipelines to remove their trigger subscriptions. This is
        /// useful if the pipeline no longer needs to be notified about a specific
        /// trigger automation.
        pub async fn unsubscribe(
            &mut self,
            request: impl tonic::IntoRequest<super::UnsubscribeRequest>,
        ) -> Result<tonic::Response<super::UnsubscribeResponse>, tonic::Status> {
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
                "/sdkProto.Trigger/Unsubscribe",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// Shutdown tells the trigger to cleanup and gracefully shutdown. If a trigger
        /// does not shutdown in a time defined by the gofer API the trigger will
        /// instead be Force shutdown(SIGKILL). This is to say that all triggers should
        /// lean toward quick cleanups and shutdowns.
        pub async fn shutdown(
            &mut self,
            request: impl tonic::IntoRequest<super::ShutdownRequest>,
        ) -> Result<tonic::Response<super::ShutdownResponse>, tonic::Status> {
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
                "/sdkProto.Trigger/Shutdown",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// ExternalEvent are json blobs of gofer's /events endpoint. Normally
        /// webhooks.
        pub async fn external_event(
            &mut self,
            request: impl tonic::IntoRequest<super::ExternalEventRequest>,
        ) -> Result<tonic::Response<super::ExternalEventResponse>, tonic::Status> {
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
                "/sdkProto.Trigger/ExternalEvent",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod trigger_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    ///Generated trait containing gRPC methods that should be implemented for use with TriggerServer.
    #[async_trait]
    pub trait Trigger: Send + Sync + 'static {
        /// Watch blocks until the trigger has a pipeline that should be run, then it
        /// returns.
        async fn watch(
            &self,
            request: tonic::Request<super::WatchRequest>,
        ) -> Result<tonic::Response<super::WatchResponse>, tonic::Status>;
        /// Info returns information on the specific plugin
        async fn info(
            &self,
            request: tonic::Request<super::InfoRequest>,
        ) -> Result<tonic::Response<super::InfoResponse>, tonic::Status>;
        /// Subscribe allows a trigger to keep track of all pipelines currently
        /// dependant on that trigger so that we can trigger them at appropriate times.
        async fn subscribe(
            &self,
            request: tonic::Request<super::SubscribeRequest>,
        ) -> Result<tonic::Response<super::SubscribeResponse>, tonic::Status>;
        /// Unsubscribe allows pipelines to remove their trigger subscriptions. This is
        /// useful if the pipeline no longer needs to be notified about a specific
        /// trigger automation.
        async fn unsubscribe(
            &self,
            request: tonic::Request<super::UnsubscribeRequest>,
        ) -> Result<tonic::Response<super::UnsubscribeResponse>, tonic::Status>;
        /// Shutdown tells the trigger to cleanup and gracefully shutdown. If a trigger
        /// does not shutdown in a time defined by the gofer API the trigger will
        /// instead be Force shutdown(SIGKILL). This is to say that all triggers should
        /// lean toward quick cleanups and shutdowns.
        async fn shutdown(
            &self,
            request: tonic::Request<super::ShutdownRequest>,
        ) -> Result<tonic::Response<super::ShutdownResponse>, tonic::Status>;
        /// ExternalEvent are json blobs of gofer's /events endpoint. Normally
        /// webhooks.
        async fn external_event(
            &self,
            request: tonic::Request<super::ExternalEventRequest>,
        ) -> Result<tonic::Response<super::ExternalEventResponse>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct TriggerServer<T: Trigger> {
        inner: _Inner<T>,
        accept_compression_encodings: (),
        send_compression_encodings: (),
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Trigger> TriggerServer<T> {
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
    impl<T, B> tonic::codegen::Service<http::Request<B>> for TriggerServer<T>
    where
        T: Trigger,
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
                "/sdkProto.Trigger/Watch" => {
                    #[allow(non_camel_case_types)]
                    struct WatchSvc<T: Trigger>(pub Arc<T>);
                    impl<T: Trigger> tonic::server::UnaryService<super::WatchRequest>
                    for WatchSvc<T> {
                        type Response = super::WatchResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::WatchRequest>,
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
                "/sdkProto.Trigger/Info" => {
                    #[allow(non_camel_case_types)]
                    struct InfoSvc<T: Trigger>(pub Arc<T>);
                    impl<T: Trigger> tonic::server::UnaryService<super::InfoRequest>
                    for InfoSvc<T> {
                        type Response = super::InfoResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::InfoRequest>,
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
                "/sdkProto.Trigger/Subscribe" => {
                    #[allow(non_camel_case_types)]
                    struct SubscribeSvc<T: Trigger>(pub Arc<T>);
                    impl<T: Trigger> tonic::server::UnaryService<super::SubscribeRequest>
                    for SubscribeSvc<T> {
                        type Response = super::SubscribeResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::SubscribeRequest>,
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
                "/sdkProto.Trigger/Unsubscribe" => {
                    #[allow(non_camel_case_types)]
                    struct UnsubscribeSvc<T: Trigger>(pub Arc<T>);
                    impl<
                        T: Trigger,
                    > tonic::server::UnaryService<super::UnsubscribeRequest>
                    for UnsubscribeSvc<T> {
                        type Response = super::UnsubscribeResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::UnsubscribeRequest>,
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
                "/sdkProto.Trigger/Shutdown" => {
                    #[allow(non_camel_case_types)]
                    struct ShutdownSvc<T: Trigger>(pub Arc<T>);
                    impl<T: Trigger> tonic::server::UnaryService<super::ShutdownRequest>
                    for ShutdownSvc<T> {
                        type Response = super::ShutdownResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ShutdownRequest>,
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
                "/sdkProto.Trigger/ExternalEvent" => {
                    #[allow(non_camel_case_types)]
                    struct ExternalEventSvc<T: Trigger>(pub Arc<T>);
                    impl<
                        T: Trigger,
                    > tonic::server::UnaryService<super::ExternalEventRequest>
                    for ExternalEventSvc<T> {
                        type Response = super::ExternalEventResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ExternalEventRequest>,
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
    impl<T: Trigger> Clone for TriggerServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }
    impl<T: Trigger> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: Trigger> tonic::transport::NamedService for TriggerServer<T> {
        const NAME: &'static str = "sdkProto.Trigger";
    }
}
