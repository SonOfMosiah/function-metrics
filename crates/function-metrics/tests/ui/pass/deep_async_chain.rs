#![allow(unused_braces)]
use function_metrics::function_metrics;

#[function_metrics]
async fn level_0() { level_1().await }

#[function_metrics]
async fn level_1() { level_2().await }

#[function_metrics]
async fn level_2() { level_3().await }

#[function_metrics]
async fn level_3() { level_4().await }

#[function_metrics]
async fn level_4() { level_5().await }

#[function_metrics]
async fn level_5() { level_6().await }

#[function_metrics]
async fn level_6() { level_7().await }

#[function_metrics]
async fn level_7() { level_8().await }

#[function_metrics]
async fn level_8() { level_9().await }

#[function_metrics]
async fn level_9() { level_10().await }

#[function_metrics]
async fn level_10() { level_11().await }

#[function_metrics]
async fn level_11() { level_12().await }

#[function_metrics]
async fn level_12() { level_13().await }

#[function_metrics]
async fn level_13() { level_14().await }

#[function_metrics]
async fn level_14() { level_15().await }

#[function_metrics]
async fn level_15() { level_16().await }

#[function_metrics]
async fn level_16() { level_17().await }

#[function_metrics]
async fn level_17() { level_18().await }

#[function_metrics]
async fn level_18() { level_19().await }

#[function_metrics]
async fn level_19() { level_20().await }

#[function_metrics]
async fn level_20() { level_21().await }

#[function_metrics]
async fn level_21() { level_22().await }

#[function_metrics]
async fn level_22() { level_23().await }

#[function_metrics]
async fn level_23() { level_24().await }

#[function_metrics]
async fn level_24() { level_25().await }

#[function_metrics]
async fn level_25() { level_26().await }

#[function_metrics]
async fn level_26() { level_27().await }

#[function_metrics]
async fn level_27() { level_28().await }

#[function_metrics]
async fn level_28() { level_29().await }

#[function_metrics]
async fn level_29() { level_30().await }

#[function_metrics]
async fn level_30() { level_31().await }

#[function_metrics]
async fn level_31() {  }

fn main() { let _ = level_0(); }
