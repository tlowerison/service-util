# service-util
A collection of utilities for writing backend web services in rust.

## Features
- anyhow
- async-graphql-4
- async-graphql-5
- async-graphql-6
- axum-05
- axum-06
- client
- color-eyre
- db
- grpc
- http1
- http2
- log_error
- max-allowed-request-body-size-lg
- max-allowed-request-body-size-md
- max-allowed-request-body-size-sm
- max-allowed-request-body-size-xl
- max-allowed-request-body-size-xxl
- server
- tracing

### Tracing
Supports the following custom environment variables for tracing configuration:
- `JAEGER_SINK_KIND: JaegerSinkKind = JaegerSinkKind::Collector`
- `LOG_HIERARCHICAL_LAYER_ANSI: Option<bool>`
- `LOG_HIERARCHICAL_LAYER_BRACKETED_FIELDS: bool = true`
- `LOG_HIERARCHICAL_LAYER_INDENT_AMOUNT: usize = 2usize`
- `LOG_HIERARCHICAL_LAYER_INDENT_LINES: bool = true`
- `LOG_HIERARCHICAL_LAYER_TARGETS: bool = true`
- `LOG_HIERARCHICAL_LAYER_THREAD_IDS: bool = false`
- `LOG_HIERARCHICAL_LAYER_THREAD_NAMES: bool = false`
- `LOG_HIERARCHICAL_LAYER_VERBOSE_ENTRY: bool = false`
- `LOG_HIERARCHICAL_LAYER_VERBOSE_EXIT: bool = false`
- `LOG_HIERARCHICAL_LAYER_WRAPAROUND: Option<usize>`
- `LOG_TARGET_DEFAULT_LEVEL: Option<tracing_subscriber::filter::LevelFilter>`
- `LOG_TARGET_FILTERS: Option<tracing_subscriber::filter::targets::Targets>`
- `OTEL_ENABLED: bool = false`
- `OTEL_EVENT_ATTRIBUTE_COUNT_LIMIT: Option<u32>`
- `OTEL_LINK_ATTRIBUTE_COUNT_LIMIT: Option<u32>`

Additional environment variables reference which are used by the opentelemetry and opentelemetry_jaeger crates:
- `OTEL_EXPORTER_JAEGER_ENDPOINT` defaults to "http://localhost:14250/api/trace"
- `OTEL_EXPORTER_JAEGER_USER`
- `OTEL_EXPORTER_JAEGER_PASSWORD`
- `OTEL_EXPORTER_JAEGER_TIMEOUT` defaults to 10 seconds
- `OTEL_SPAN_ATTRIBUTE_COUNT_LIMIT` defaults to 128
- `OTEL_SPAN_EVENT_COUNT_LIMIT` defaults to 128
- `OTEL_SPAN_LINK_COUNT_LIMIT` defaults to 128
- `OTEL_TRACES_SAMPLER` defaults to "parentbased_always_on"
- `OTEL_TRACES_SAMPLER_ARG`
- `OTEL_BSP_SCHEDULE_DELAY` defaults to 5 seconds
- `OTEL_BSP_MAX_QUEUE_SIZE` defaults to 2048
- `OTEL_BSP_MAX_EXPORT_BATCH_SIZE` defaults to 512
- `OTEL_BSP_EXPORT_TIMEOUT` defaults to 30 seconds
- `OTEL_BSP_MAX_CONCURRENT_EXPORTS` defaults to 1

