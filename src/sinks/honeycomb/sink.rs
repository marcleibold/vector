//! Implementation of the `honeycomb` sink.

use crate::sinks::{
    prelude::*,
    util::http::{HttpJsonBatchSizer, HttpRequest},
};

use super::request_builder::HoneycombRequestBuilder;

pub(super) struct HoneycombSink<S> {
    service: S,
    batch_settings: BatcherSettings,
    request_builder: HoneycombRequestBuilder,
}

impl<S> HoneycombSink<S>
where
    S: Service<HttpRequest> + Send + 'static,
    S::Future: Send + 'static,
    S::Response: DriverResponse + Send + 'static,
    S::Error: std::fmt::Debug + Into<crate::Error> + Send,
{
    /// Creates a new `HoneycombSink`.
    pub(super) const fn new(
        service: S,
        batch_settings: BatcherSettings,
        request_builder: HoneycombRequestBuilder,
    ) -> Self {
        Self {
            service,
            batch_settings,
            request_builder,
        }
    }

    async fn run_inner(self: Box<Self>, input: BoxStream<'_, Event>) -> Result<(), ()> {
        input
            // Batch the input stream with size calculation based on the estimated encoded json size
            .batched(
                self.batch_settings
                    .into_item_size_config(HttpJsonBatchSizer),
            )
            // Build requests with no concurrency limit.
            .request_builder(None, self.request_builder)
            // Filter out any errors that occurred in the request building.
            .filter_map(|request| async move {
                match request {
                    Err(error) => {
                        emit!(SinkRequestBuildError { error });
                        None
                    }
                    Ok(req) => Some(req),
                }
            })
            // Generate the driver that will send requests and handle retries,
            // event finalization, and logging/internal metric reporting.
            .into_driver(self.service)
            .run()
            .await
    }
}

#[async_trait::async_trait]
impl<S> StreamSink<Event> for HoneycombSink<S>
where
    S: Service<HttpRequest> + Send + 'static,
    S::Future: Send + 'static,
    S::Response: DriverResponse + Send + 'static,
    S::Error: std::fmt::Debug + Into<crate::Error> + Send,
{
    async fn run(
        self: Box<Self>,
        input: futures_util::stream::BoxStream<'_, Event>,
    ) -> Result<(), ()> {
        self.run_inner(input).await
    }
}
