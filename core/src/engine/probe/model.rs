//! Probe data model.

use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Default)]
pub struct DeviceInfo {
    pub device: String,
    pub model: String,
    pub rom: String,
    pub soc_id: String,
    pub kernel: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct EntryState {
    pub exists: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Options {
    /// policy -> available governors
    pub governors: BTreeMap<String, Vec<String>>,
    /// policy -> available frequencies (ascending, kHz)
    pub freqs: BTreeMap<String, Vec<u64>>,
    pub gpu_governors: Vec<String>,
    /// number of GPU pwrlevels (valid indices 0..n)
    pub gpu_levels: usize,
    /// offered block schedulers (current one marked by caller if needed)
    pub io_schedulers: Vec<String>,
    pub tcp_cc: Vec<String>,
    /// number of online-capable CPUs (0..n usable in masks)
    pub cpu_count: usize,
    /// policy -> number of CPUs in the cluster (core_ctl task_thres bound:
    /// kernel rejects val < num_cpus, core_ctl.c store_task_thres).
    pub cluster_cpus: BTreeMap<String, usize>,
    /// policy -> live core_ctl max_cpus (min_cpus is clamped to it by the
    /// kernel: core_ctl.c store_min_cpus `min(val, state->max_cpus)`).
    pub core_ctl_max: BTreeMap<String, i64>,
}

/// Read-only evidence of framework-owned nodes — proves coexistence and
/// gives the UI something to display ("MIUI daemons: running").
#[derive(Debug, Clone, Serialize, Default)]
pub struct FrameworkEvidence {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mi_thermald: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub perf_hal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub perfservice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thermal_sconfig: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msm_perf_locks: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_boost: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sched_boost: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_cpuset: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProbeData {
    pub device: DeviceInfo,
    pub entries: BTreeMap<String, EntryState>,
    pub options: Options,
    pub framework: FrameworkEvidence,
}
