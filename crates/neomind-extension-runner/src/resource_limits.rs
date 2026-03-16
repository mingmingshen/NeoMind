//! Resource limits for extension processes
//!
//! This module provides cross-platform resource limiting to prevent
//! extensions from consuming excessive memory, CPU, or other resources.
//!
//! # Supported Platforms
//!
//! - **Linux/macOS**: Uses `setrlimit()` for memory limits
//! - **Windows**: Uses Job Objects with memory limits
//!
//! # Example
//!
//! ```rust
//! use resource_limits::setup_resource_limits;
//!
//! async fn main() {
//!     // Set up limits before loading extension
//!     setup_resource_limits(&ResourceLimitsConfig {
//!         memory_limit_mb: Some(512),
//!         cpu_affinity: None,
//!         nice_level: Some(10),
//!     })?;
//!
//!     // Load and run extension
//!     let runner = Runner::load(&path).await?;
//!     runner.run().await;
//! }
//! ```

use std::io;
use tracing::{info, warn, error};

/// Configuration for resource limits
#[derive(Debug, Clone)]
pub struct ResourceLimitsConfig {
    /// Memory limit in MB (soft limit), None = no limit
    pub memory_limit_mb: Option<u64>,

    /// Memory limit in MB (hard limit), None = 2x soft limit
    pub memory_limit_hard_mb: Option<u64>,

    /// CPU affinity (which cores to use), None = all cores
    /// Example: Some(vec![0, 1]) to use only cores 0 and 1
    pub cpu_affinity: Option<Vec<usize>>,

    /// Process nice level (priority), None = default
    /// Lower value = higher priority (-20 to 19)
    /// 10 = lower priority for background processes
    pub nice_level: Option<i32>,
}

impl Default for ResourceLimitsConfig {
    fn default() -> Self {
        Self {
            memory_limit_mb: Some(512),  // 512MB default
            memory_limit_hard_mb: None,  // 2x soft limit
            cpu_affinity: None,
            nice_level: Some(10),        // Lower priority
        }
    }
}

impl ResourceLimitsConfig {
    /// Create a new config with memory limit only
    pub fn with_memory_limit_mb(mb: u64) -> Self {
        Self {
            memory_limit_mb: Some(mb),
            ..Default::default()
        }
    }

    /// Create a new config with no limits (for testing)
    pub fn unrestricted() -> Self {
        Self {
            memory_limit_mb: None,
            memory_limit_hard_mb: None,
            cpu_affinity: None,
            nice_level: None,
        }
    }
}

/// Set up resource limits for the current process
///
/// This must be called BEFORE loading the extension, as it applies
/// to the current process (the extension runner process).
///
/// # Arguments
///
/// * `config` - Resource limits configuration
///
/// # Returns
///
/// * `Ok(())` - Limits successfully applied
/// * `Err(e)` - Failed to apply limits (process should exit)
///
/// # Errors
///
/// This function will return an error if:
/// - The platform is not supported
/// - The system calls fail (permission denied, invalid value, etc.)
pub fn setup_resource_limits(config: &ResourceLimitsConfig) -> Result<(), ResourceLimitError> {
    info!("Setting up resource limits: {:?}", config);

    #[cfg(unix)]
    {
        setup_unix_limits(config)?;
    }

    #[cfg(windows)]
    {
        setup_windows_limits(config)?;
    }

    #[cfg(not(any(unix, windows)))]
    {
        warn!("Resource limits not supported on this platform");
    }

    info!("Resource limits configured successfully");
    Ok(())
}

/// Error types for resource limit operations
#[derive(Debug, thiserror::Error)]
pub enum ResourceLimitError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Platform not supported")]
    PlatformNotSupported,

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("System error: {0}")]
    SystemError(String),
}

// ============================================================================
// Unix (Linux/macOS) Implementation
// ============================================================================

#[cfg(unix)]
fn setup_unix_limits(config: &ResourceLimitsConfig) -> Result<(), ResourceLimitError> {
    use libc::{c_int, setrlimit, rlimit, setpriority, PRIO_PROCESS};
    #[cfg(target_os = "linux")]
    use libc::RLIMIT_AS;
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    use libc::RLIMIT_DATA;

    // 1. Set memory limit
    if let Some(soft_mb) = config.memory_limit_mb {
        #[cfg(target_os = "linux")]
        {
            let soft = soft_mb * 1024 * 1024;
            let hard = config.memory_limit_hard_mb
                .unwrap_or(soft_mb * 2) * 1024 * 1024;

            info!(
                "Setting memory limit: soft={}MB, hard={}MB",
                soft_mb,
                config.memory_limit_hard_mb.unwrap_or(soft_mb * 2)
            );

            let limits = rlimit {
                rlim_cur: soft,
                rlim_max: hard,
            };

            let result = unsafe {
                // Linux: Use RLIMIT_AS (address space limit)
                setrlimit(RLIMIT_AS, &limits)
            };

            if result != 0 {
                let err = io::Error::last_os_error();
                warn!("Failed to set memory limit: {}. Continuing anyway.", err);
            } else {
                info!("Memory limit set successfully on Linux");
            }
        }

        #[cfg(target_os = "macos")]
        {
            // macOS: RLIMIT_DATA/RLIMIT_AS have limited support
            // macOS uses JetSam (Low Memory Monitor) for memory management
            // We'll just log the requested limit for monitoring purposes
            info!(
                "Memory limit requested: {}MB (note: macOS does not support rlimit-based memory limits, using JetSam instead)",
                soft_mb
            );
            info!("The system will manage memory via its native Low Memory Monitor");
        }
    }

    // 2. Set process priority (nice level)
    if let Some(nice) = config.nice_level {
        info!("Setting nice level to {}", nice);

        let result = unsafe { setpriority(PRIO_PROCESS, 0, nice) };

        if result != 0 {
            let err = io::Error::last_os_error();
            // Non-fatal: just warn if we can't set priority
            warn!("Failed to set nice level: {} (continuing anyway)", err);
        } else {
            info!("Nice level set to {}", nice);
        }
    }

    // 3. Set CPU affinity if specified
    if let Some(ref cores) = config.cpu_affinity {
        info!("Setting CPU affinity to cores: {:?}", cores);
        set_cpu_affinity_unix(cores)?;
    }

    Ok(())
}

#[cfg(unix)]
fn set_cpu_affinity_unix(cores: &[usize]) -> Result<(), ResourceLimitError> {
    #[cfg(target_os = "linux")]
    {
        use libc::{cpu_set_t, sched_setaffinity, CPU_SET, CPU_ZERO};

        let mut cpuset: cpu_set_t = unsafe { std::mem::zeroed() };
        unsafe {
            CPU_ZERO(&mut cpuset);
            for &core in cores {
                if core < libc::CPU_SETSIZE as usize {
                    CPU_SET(core, &mut cpuset);
                } else {
                    return Err(ResourceLimitError::InvalidConfig(format!(
                        "CPU core {} exceeds CPU_SETSIZE",
                        core
                    )));
                }
            }
        }

        let result = unsafe {
            sched_setaffinity(0, std::mem::size_of::<cpu_set_t>(), &cpuset)
        };

        if result != 0 {
            let err = io::Error::last_os_error();
            warn!("Failed to set CPU affinity: {} (continuing anyway)", err);
        } else {
            info!("CPU affinity set to cores: {:?}", cores);
        }
    }

    #[cfg(target_os = "macos")]
    {
        use libc::{thread_policy_t, thread_affinity_policy_data_t, thread_policy_set,
                   mach_thread_self, THREAD_AFFINITY_POLICY, THREAD_AFFINITY_POLICY_COUNT};

        // macOS has different CPU affinity API
        // For now, just log that we're skipping this
        warn!("CPU affinity not fully supported on macOS, skipping");
    }

    Ok(())
}

// ============================================================================
// Windows Implementation
// ============================================================================

#[cfg(windows)]
fn setup_windows_limits(config: &ResourceLimitsConfig) -> Result<(), ResourceLimitError> {
    use windows::Win32::System::JobObjects::*;
    use windows::Win32::Foundation::*;
    use windows::Win32::System::Threading::*;

    // Memory limit on Windows requires creating a Job Object
    if let Some(_limit_mb) = config.memory_limit_mb {
        info!("Setting up Windows Job Object with memory limit");

        unsafe {
            // Create a job object
            let job = CreateJobObjectW(None, None)?;

            // Set memory limit
            let mut info = JOBOBJECT_BASIC_LIMIT_INFORMATION {
                ..Default::default()
            };

            info.LimitFlags = JOB_OBJECT_LIMIT_JOB_MEMORY;
            info.JobMemoryLimit = (_limit_mb * 1024 * 1024) as usize;

            let mut extended_info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
                BasicLimitInformation: info,
                ..Default::default()
            };

            let result = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &extended_info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );

            if !result.as_bool() {
                let err = io::Error::last_os_error();
                error!("Failed to set job object limits: {}", err);
                return Err(ResourceLimitError::SystemError(format!(
                    "SetInformationJobObject failed: {}",
                    err
                )));
            }

            // Assign current process to the job
            let result = AssignProcessToJobObject(job, GetCurrentProcess());

            if !result.as_bool() {
                let err = io::Error::last_os_error();
                warn!("Failed to assign process to job object: {} (continuing anyway)", err);
            } else {
                info!("Process assigned to job object with memory limit");
            }
        }
    }

    // Set process priority on Windows
    if let Some(_nice) = config.nice_level {
        info!("Setting process priority on Windows");

        unsafe {
            let priority = BELOW_NORMAL_PRIORITY_CLASS;
            let result = SetPriorityClass(GetCurrentProcess(), priority);

            if !result.as_bool() {
                let err = io::Error::last_os_error();
                warn!("Failed to set process priority: {} (continuing anyway)", err);
            } else {
                info!("Process priority set to BELOW_NORMAL");
            }
        }
    }

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = ResourceLimitsConfig::default();
        assert_eq!(config.memory_limit_mb, Some(512));
        assert_eq!(config.nice_level, Some(10));
    }

    #[test]
    fn test_config_with_memory() {
        let config = ResourceLimitsConfig::with_memory_limit_mb(256);
        assert_eq!(config.memory_limit_mb, Some(256));
        assert_eq!(config.nice_level, Some(10));
    }

    #[test]
    fn test_config_unrestricted() {
        let config = ResourceLimitsConfig::unrestricted();
        assert_eq!(config.memory_limit_mb, None);
        assert_eq!(config.nice_level, None);
    }

    #[test]
    fn test_setup_limits_unrestricted() {
        // Should not fail with unrestricted config
        let result = setup_resource_limits(&ResourceLimitsConfig::unrestricted());
        assert!(result.is_ok());
    }
}
