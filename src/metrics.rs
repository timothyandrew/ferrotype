//! In-memory metrics database, modeled as a mutable `HashMap` behind a
//! immutable singleton via `lazy_static`. Needs to be manually flushed.

use std::collections::HashMap;
use std::sync::Mutex;

lazy_static! {
  static ref METRICS: Mutex<HashMap<String, i32>> = {
      println!("Initializing metrics store…");
      Mutex::new(HashMap::new())
  };
}

pub fn init() -> &'static Mutex<HashMap<String, i32>> {
    &METRICS
}

pub fn tick(metric: &str) {
    let mut handle = METRICS.lock().unwrap();

    if let Some(value) = handle.get_mut(metric) {
        *value = *value + 1;
    } else {
        handle.insert(metric.to_string(), 1);
    }
}

/// Flush to `stdout` for now, but this could potentially flush to something
/// like influx/grafana eventually.
pub fn flush() {
    println!("{:?}", METRICS.lock().unwrap());
}