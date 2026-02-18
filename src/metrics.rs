use lazy_static::lazy_static;
use prometheus::{
    CounterVec, HistogramVec, Registry, opts, register_counter_vec_with_registry,
    register_histogram_vec_with_registry,
};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref REQUEST_COUNTER: CounterVec = register_counter_vec_with_registry!(
        opts!(
            "mimicrab_requests_total",
            "Total number of requests handled by Mimicrab"
        ),
        &["matched", "path"],
        REGISTRY
    )
    .unwrap();
    pub static ref REQUEST_DURATION: HistogramVec = register_histogram_vec_with_registry!(
        "mimicrab_request_duration_seconds",
        "Histogram of request latencies in seconds",
        &["path"],
        REGISTRY
    )
    .unwrap();
}

pub fn register_process_metrics() {
    #[cfg(target_os = "linux")]
    {
        let process_collector = prometheus::process_collector::ProcessCollector::for_self();
        REGISTRY.register(Box::new(process_collector)).unwrap();
    }
}
