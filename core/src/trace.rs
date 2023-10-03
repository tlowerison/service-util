use crate::env;
use ::chrono::Utc;
use ::hyper::header::HeaderName;
use ::hyper::Request;
use ::opentelemetry::propagation::TextMapPropagator;
use ::opentelemetry::sdk::propagation::TraceContextPropagator;
use ::opentelemetry::trace::TraceContextExt;
use ::serde::*;
use ::std::collections::HashMap;
use ::tracing::Span;
use ::tracing_error::ErrorLayer;
use ::tracing_log::LogTracer;
use ::tracing_opentelemetry::{OpenTelemetryLayer, OpenTelemetrySpanExt};
use ::tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use ::tracing_tree::{time::FormatTime, HierarchicalLayer};

#[derive(Clone, Copy, Debug)]
pub struct UTCTime;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JaegerSinkKind {
    Agent,
    Collector,
}

#[macro_export]
macro_rules! instrument_field {
    ($ident:ident) => {
        tracing::Span::current().record(stringify!($ident), &&*format!("{:?}", $ident));
    };
    ($name:literal, $expr:expr) => {
        tracing::Span::current().record($name, &&*format!("{:?}", $expr));
    };
}

// macro because typically $registry is a deeply nested type
// which would have an unknown amount of required type parameters
// if being accepted as a function argument
macro_rules! set_global_default {
    ($registry:expr) => {
        tracing::subscriber::set_global_default($registry).expect("failed to set subscriber")
    };
}

pub static TRACEPARENT: HeaderName = HeaderName::from_static("traceparent");
pub static TRACESTATE: HeaderName = HeaderName::from_static("tracestate");

env! {
    JAEGER_SINK_KIND: JaegerSinkKind = JaegerSinkKind::Collector,
    LOG_HIERARCHICAL_LAYER_ANSI: Option<bool>,
    LOG_HIERARCHICAL_LAYER_BRACKETED_FIELDS: bool = true,
    LOG_HIERARCHICAL_LAYER_INDENT_AMOUNT: usize = 2usize,
    LOG_HIERARCHICAL_LAYER_INDENT_LINES: bool = true,
    LOG_HIERARCHICAL_LAYER_TARGETS: bool = true,
    LOG_HIERARCHICAL_LAYER_THREAD_IDS: bool = false,
    LOG_HIERARCHICAL_LAYER_THREAD_NAMES: bool = false,
    LOG_HIERARCHICAL_LAYER_VERBOSE_ENTRY: bool = false,
    LOG_HIERARCHICAL_LAYER_VERBOSE_EXIT: bool = false,
    LOG_HIERARCHICAL_LAYER_WRAPAROUND: Option<usize>,
    OTEL_EVENT_ATTRIBUTE_COUNT_LIMIT: Option<u32>,
    OTEL_LINK_ATTRIBUTE_COUNT_LIMIT: Option<u32>,
    TELEMETRY_ENABLED: bool = false,
    // OpenTelemetry environment variables which are already handled by
    // the opentelemetry and opentelemetry_jaeger libraries:
    // - OTEL_EXPORTER_JAEGER_ENDPOINT: defaults to "http://localhost:14250/api/trace"
    // - OTEL_EXPORTER_JAEGER_USER
    // - OTEL_EXPORTER_JAEGER_PASSWORD
    // - OTEL_EXPORTER_JAEGER_TIMEOUT: defaults to 10 seconds
    // - OTEL_SPAN_ATTRIBUTE_COUNT_LIMIT: defaults to 128
    // - OTEL_SPAN_EVENT_COUNT_LIMIT: defaults to 128
    // - OTEL_SPAN_LINK_COUNT_LIMIT: defaults to 128
    // - OTEL_TRACES_SAMPLER: defaults to "parentbased_always_on"
    // - OTEL_TRACES_SAMPLER_ARG:
    // - OTEL_BSP_SCHEDULE_DELAY: defaults to 5 seconds
    // - OTEL_BSP_MAX_QUEUE_SIZE: defaults to 2048
    // - OTEL_BSP_MAX_EXPORT_BATCH_SIZE: defaults to 512
    // - OTEL_BSP_EXPORT_TIMEOUT: defaults to 30 seconds
    // - OTEL_BSP_MAX_CONCURRENT_EXPORTS: defaults to 1
}

pub fn install_tracing() {
    LogTracer::init().expect("unable to initialize LogTracer");

    let registry = Registry::default()
        .with(EnvFilter::from_default_env())
        .with(hierarchical_layer());

    if telemetry_enabled().unwrap() {
        set_global_default!(registry
            .with(OpenTelemetryLayer::new(jaeger_tracer()))
            .with(ErrorLayer::default()));
    } else {
        set_global_default!(registry.with(ErrorLayer::default()));
    }

    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());
}

fn hierarchical_layer() -> HierarchicalLayer<fn() -> std::io::Stderr, UTCTime> {
    let mut layer = ::tracing_tree::HierarchicalLayer::new(log_hierarchical_layer_indent_amount().unwrap())
        .with_bracketed_fields(log_hierarchical_layer_bracketed_fields().unwrap())
        .with_indent_lines(log_hierarchical_layer_indent_lines().unwrap())
        .with_targets(log_hierarchical_layer_targets().unwrap())
        .with_thread_ids(log_hierarchical_layer_thread_ids().unwrap())
        .with_thread_names(log_hierarchical_layer_thread_names().unwrap())
        .with_verbose_entry(log_hierarchical_layer_verbose_entry().unwrap())
        .with_verbose_exit(log_hierarchical_layer_verbose_exit().unwrap())
        .with_timer(UTCTime);

    if let Some(ansi) = log_hierarchical_layer_ansi().unwrap() {
        layer = layer.with_ansi(ansi);
    }
    if let Some(wraparound) = log_hierarchical_layer_wraparound().unwrap() {
        layer = layer.with_wraparound(wraparound);
    }
    layer
}

fn jaeger_tracer() -> opentelemetry::sdk::trace::Tracer {
    let mut config = opentelemetry::sdk::trace::Config::default();

    if let Some(max_attributes) = otel_event_attribute_count_limit().unwrap() {
        config = config.with_max_attributes_per_event(max_attributes);
    }
    if let Some(max_attributes) = otel_link_attribute_count_limit().unwrap() {
        config = config.with_max_attributes_per_link(max_attributes);
    }

    match jaeger_sink_kind().unwrap() {
        JaegerSinkKind::Agent => opentelemetry_jaeger::new_agent_pipeline()
            .with_trace_config(config)
            .install_batch(opentelemetry::runtime::Tokio)
            .expect("unable to install Jaeger Agent pipeline"),
        JaegerSinkKind::Collector => opentelemetry_jaeger::new_collector_pipeline()
            .with_trace_config(config)
            .install_batch(opentelemetry::runtime::Tokio)
            .expect("unable to install Jaeger Collector pipeline"),
    }
}

pub fn set_trace_parent(req: &Request<hyper::Body>, span: Span) -> Span {
    let propagator = TraceContextPropagator::new();
    if let Some(traceparent) = req.headers().get(&TRACEPARENT).and_then(|x| x.to_str().ok()) {
        // Propagator::extract only works with HashMap<String, String>
        let mut headers = match req.headers().get(&TRACESTATE).and_then(|x| x.to_str().ok()) {
            Some(tracestate) => {
                let mut headers = HashMap::with_capacity(2);
                headers.insert("tracestate".to_string(), tracestate.to_string());
                headers
            }
            None => HashMap::with_capacity(1),
        };
        headers.insert("traceparent".to_string(), traceparent.to_string());

        let context = propagator.extract(&headers);
        span.set_parent(context);
    }
    span
}

pub fn traceparent() -> Option<String> {
    let context = Span::current().context();
    let span_ref = context.span();
    let span_context = span_ref.span_context();
    if !span_context.is_valid() {
        return None;
    }
    let trace_id = span_context.trace_id();
    let span_id = span_context.span_id();
    let flags = span_context.trace_flags().to_u8();
    Some(format!("00-{trace_id}-{span_id}-{flags:02x}"))
}

impl FormatTime for UTCTime {
    fn format_time(&self, w: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(w, "{}", Utc::now().format("%+"))
    }
}

impl std::str::FromStr for JaegerSinkKind {
    type Err = ::anyhow::Error;
    fn from_str(str: &str) -> Result<Self, ::anyhow::Error> {
        Ok(match str {
            "agent" => Self::Agent,
            "collector" => Self::Collector,
            _ => {
                return Err(::anyhow::Error::msg(format!(
                    "unrecognized JaegerSinkKind variant: {str}"
                )))
            }
        })
    }
}
