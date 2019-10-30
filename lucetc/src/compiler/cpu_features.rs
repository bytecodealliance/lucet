use crate::error::{LucetcError, LucetcErrorKind};
use cranelift_codegen::{isa, settings::Configurable};
use failure::{format_err, ResultExt};
use target_lexicon::Triple;

/// x86 CPU families used as shorthand for different CPU feature configurations.
///
/// Matches the definitions from `cranelift-codegen`'s x86 settings definition.
#[derive(Debug, Clone, Copy)]
pub enum TargetCpu {
    Baseline,
    Nehalem,
    Haswell,
    Broadwell,
    Skylake,
    Cannonlake,
    Icelake,
    Znver1,
}

/// A manual specification of the CPU features to use during codegen.
#[derive(Debug, Clone, Copy)]
pub struct SpecificFeatures {
    pub has_sse3: bool,
    pub has_ssse3: bool,
    pub has_sse41: bool,
    pub has_sse42: bool,
    pub has_popcnt: bool,
    pub has_avx: bool,
    pub has_bmi1: bool,
    pub has_bmi2: bool,
    pub has_lzcnt: bool,
}

impl SpecificFeatures {
    /// Return a `CpuFeatures` with all optional features disabled.
    pub fn all_disabled() -> Self {
        Self {
            has_sse3: false,
            has_ssse3: false,
            has_sse41: false,
            has_sse42: false,
            has_popcnt: false,
            has_avx: false,
            has_bmi1: false,
            has_bmi2: false,
            has_lzcnt: false,
        }
    }
}

impl From<TargetCpu> for SpecificFeatures {
    fn from(cpu: TargetCpu) -> Self {
        use TargetCpu::*;
        match cpu {
            Baseline => Self::all_disabled(),
            Nehalem => Self {
                has_sse3: true,
                has_ssse3: true,
                has_sse41: true,
                has_sse42: true,
                has_popcnt: true,
                ..Baseline.into()
            },
            Haswell => Self {
                // Note: this is not part of the Cranelift profile for Haswell, which only uses
                // CPUID detection to enable AVX. If we want to bypass CPUID when compiling, we need
                // to set it manually, and Haswell is the first of the CPUs with profiles to have
                // AVX.
                has_avx: true,
                has_bmi1: true,
                has_bmi2: true,
                has_lzcnt: true,
                ..Nehalem.into()
            },
            Broadwell => Haswell.into(),
            Skylake => Broadwell.into(),
            Cannonlake => Skylake.into(),
            Icelake => Cannonlake.into(),
            Znver1 => Self {
                has_sse3: true,
                has_ssse3: true,
                has_sse41: true,
                has_sse42: true,
                has_popcnt: true,
                // Note: similarly to the Haswell AVX flag, we don't want to rely on CPUID detection
                // here, but Ryzen does support AVX.
                has_avx: true,
                has_bmi1: true,
                has_bmi2: true,
                has_lzcnt: true,
            },
        }
    }
}

/// x86-specific CPU features that affect code generation.
#[derive(Debug, Clone, Copy)]
pub enum CpuFeatures {
    /// Detect and use the CPU features available on the host at compile-time.
    DetectCpuid,
    /// Use specific CPU features rather than relying on CPUID detection.
    Specify(SpecificFeatures),
}

impl Default for CpuFeatures {
    fn default() -> Self {
        Self::detect_cpuid()
    }
}

impl CpuFeatures {
    /// Return a `CpuFeatures` that uses the CPUID instruction to determine which features to enable.
    pub fn detect_cpuid() -> Self {
        CpuFeatures::DetectCpuid
    }

    /// Return a `CpuFeatures` with no optional features enabled.
    pub fn all_disabled() -> Self {
        CpuFeatures::Specify(SpecificFeatures::all_disabled())
    }

    /// Return a `cranelift_codegen::isa::Builder` configured with these CPU features.
    pub fn isa_builder(&self) -> Result<isa::Builder, LucetcError> {
        match self {
            CpuFeatures::DetectCpuid => cranelift_native::builder()
                .map_err(|_| format_err!("host machine is not a supported target"))
                .context(LucetcErrorKind::Unsupported)
                .map_err(|e| e.into()),
            CpuFeatures::Specify(features) => {
                let mut isa_builder = isa::lookup(Triple::host())
                    .map_err(|_| format_err!("host machine is not a supported target"))
                    .context(LucetcErrorKind::Unsupported)?;

                macro_rules! enable_feature {
                    ( $feature:ident ) => {
                        if features.$feature {
                            isa_builder
                                .enable(stringify!($feature))
                                .context(LucetcErrorKind::Unsupported)?;
                        }
                    };
                }

                enable_feature!(has_sse3);
                enable_feature!(has_ssse3);
                enable_feature!(has_sse41);
                enable_feature!(has_sse42);
                enable_feature!(has_popcnt);
                enable_feature!(has_avx);
                enable_feature!(has_bmi1);
                enable_feature!(has_bmi2);
                enable_feature!(has_lzcnt);

                Ok(isa_builder)
            }
        }
    }
}
