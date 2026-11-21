//! Catalog entry macros.

macro_rules! e {
    ($key:expr, $path:expr, $tier:expr, $kind:expr, $scope:expr) => {
        Entry {
            key: $key,
            path: $path,
            tier: $tier,
            kind: $kind,
            scope: $scope,
            range: None,
            min_cpus_of: None,
        }
    };
}

/// Entry with a kernel-confirmed inclusive integer range.
macro_rules! er {
    ($key:expr, $path:expr, $tier:expr, $kind:expr, $scope:expr, $min:expr, $max:expr) => {
        Entry {
            key: $key,
            path: $path,
            tier: $tier,
            kind: $kind,
            scope: $scope,
            range: Some(($min, $max)),
            min_cpus_of: None,
        }
    };
}

/// Entry validated against a cluster's CPU count (core_ctl task_thres).
macro_rules! et {
    ($key:expr, $path:expr, $tier:expr, $cluster:expr) => {
        Entry {
            key: $key,
            path: $path,
            tier: $tier,
            kind: Kind::Int,
            scope: "",
            range: None,
            min_cpus_of: Some($cluster),
        }
    };
}

/// Per-scheduler queue tunable (`/queue/iosched/...`, pass-2 handled).
macro_rules! ei {
    ($key:expr, $path:expr, $tier:expr, $min:expr, $max:expr) => {
        Entry {
            key: $key,
            path: $path,
            tier: $tier,
            kind: Kind::Int,
            scope: "",
            range: Some(($min, $max)),
            min_cpus_of: None,
        }
    };
}
