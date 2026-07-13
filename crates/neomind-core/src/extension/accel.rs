//! Extension hardware variant detection + fallback chain.
//!
//! Single source of truth for selecting the right extension build variant
//! (cpu / cuda / jetson) on the **download side** (marketplace metadata
//! `builds` keys). The `.nep` internal structure is variant-agnostic —
//! variant discrimination lives only in marketplace `builds` keys and
//! release filenames.

/// Hardware variant of the current device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    Cpu,
    Cuda,
    Jetson,
}

impl Variant {
    /// Suffix appended to a hyphen-format platform key (matches metadata `builds` keys).
    /// `Cpu` has no suffix (the base platform key itself).
    pub fn suffix_hyphen(&self) -> Option<&'static str> {
        match self {
            Variant::Cpu => None,
            Variant::Cuda => Some("cuda"),
            Variant::Jetson => Some("jetson"),
        }
    }
}

/// Fallback candidate keys, most specific → most general (no `wasm`; caller appends it).
/// Examples:
///   ("linux-aarch64", Jetson) → ["linux-aarch64-jetson", "linux-aarch64"]
///   ("linux-aarch64", Cpu)    → ["linux-aarch64"]
pub fn fallback_keys(base_hyphen: &str, variant: Variant) -> Vec<String> {
    let mut keys = Vec::new();
    if let Some(suffix) = variant.suffix_hyphen() {
        keys.push(format!("{}-{}", base_hyphen, suffix));
    }
    keys.push(base_hyphen.to_string());
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suffix_hyphen_values() {
        assert_eq!(Variant::Cpu.suffix_hyphen(), None);
        assert_eq!(Variant::Cuda.suffix_hyphen(), Some("cuda"));
        assert_eq!(Variant::Jetson.suffix_hyphen(), Some("jetson"));
    }

    #[test]
    fn fallback_keys_jetson() {
        let keys = fallback_keys("linux-aarch64", Variant::Jetson);
        assert_eq!(keys, vec!["linux-aarch64-jetson", "linux-aarch64"]);
    }

    #[test]
    fn fallback_keys_cuda() {
        let keys = fallback_keys("linux-x86_64", Variant::Cuda);
        assert_eq!(keys, vec!["linux-x86_64-cuda", "linux-x86_64"]);
    }

    #[test]
    fn fallback_keys_cpu_has_no_variant_suffix() {
        let keys = fallback_keys("linux-aarch64", Variant::Cpu);
        assert_eq!(keys, vec!["linux-aarch64"]);
    }

    #[test]
    fn fallback_keys_cpu_darwin() {
        let keys = fallback_keys("darwin-aarch64", Variant::Cpu);
        assert_eq!(keys, vec!["darwin-aarch64"]);
    }
}
