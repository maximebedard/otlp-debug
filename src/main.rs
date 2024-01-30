use std::{panic, time::Duration};

use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::config;
use opentelemetry_sdk::Resource;
use tokio::time::sleep;
use tracing::{info_span, instrument, level_filters::LevelFilter, Instrument};
use tracing_opentelemetry::layer;
use tracing_subscriber::{fmt, EnvFilter};
use tracing_subscriber::{layer::SubscriberExt, Registry};

fn init_telemetry() {
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint("http://127.0.0.1:4317")
        .with_timeout(Duration::from_secs(3));

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_trace_config(config().with_resource(Resource::new(vec![
            KeyValue::new("deployment.environment", "dev"),
            KeyValue::new("service.name", env!("CARGO_CRATE_NAME")),
        ])))
        .with_exporter(exporter)
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .unwrap();

    let subscriber = Registry::default()
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::ERROR.into())
                .parse(concat!(env!("CARGO_CRATE_NAME"), "=info"))
                .unwrap(),
        )
        .with(fmt::layer())
        .with(layer().with_tracer(tracer));

    tracing::subscriber::set_global_default(subscriber).unwrap();
}

fn shutdown_telemetry() {
    global::shutdown_tracer_provider();
}

#[tokio::main]
async fn main() {
    init_telemetry();

    match tokio::spawn(do_something().instrument(info_span!("test"))).await {
        Ok(()) => {}
        Err(err) if err.is_panic() => panic::resume_unwind(err.into_panic()),
        Err(err) => panic!("{}", err),
    }

    shutdown_telemetry()
}

#[instrument]
async fn do_something() {
    tracing::info!("trace-bbb");

    async {
        sleep(Duration::from_millis(500))
            .instrument(info_span!("bbb"))
            .await;

        tracing::info!("trace-bbb");

        async {
            sleep(Duration::from_millis(250))
                .instrument(info_span!("ddd"))
                .await;

            tracing::info!("trace-ccc");

            sleep(Duration::from_millis(250))
                .instrument(info_span!("eee"))
                .await;
        }
        .instrument(info_span!("ccc"))
        .await;
    }
    .instrument(info_span!("aaa"))
    .await;

    sleep(Duration::from_millis(250))
        .instrument(info_span!("fff"))
        .await;
}
