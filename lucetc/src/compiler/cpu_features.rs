use crate::error::Error;
use cranelift_codegen::{isa, settings::Configurable};
use lucet_module::ModuleFeatures;
use std::collections::{HashMap, HashSet};
use target_lexicon::Triple;

use raw_cpuid::CpuId;

/// x86 CPU families used as shorthand for different CPU feature configurations.
///
/// Matches the definitions from `cranelift-codegen`'s x86 settings definition.
#[derive(Debug, Clone, Copy)]
pub enum TargetCpu {
    Native,
    Baseline,
    Nehalem,
    Sandybridge,
    Haswell,
    Broadwell,
    Skylake,
    Cannonlake,
    Icelake,
    Znver1,
}

impl TargetCpu {
    fn features(&self) -> Vec<SpecificFeature> {
        use SpecificFeature::*;
        use TargetCpu::*;
        match self {
            Native | Baseline => vec![],
            Nehalem => vec![SSE3, SSSE3, SSE41, SSE42, Popcnt],
            // Note: this is not part of the Cranelift profile for Haswell, and there is no Sandy
            // Bridge profile. Instead, Cranelift only uses CPUID detection to enable AVX. If we
            // want to bypass CPUID when compiling, we need to set AVX manually, and Sandy Bridge is
            // the first family of Intel CPUs with AVX.
            Sandybridge => [Nehalem.features().as_slice(), &[AVX]].concat(),
            Haswell => [Sandybridge.features().as_slice(), &[BMI1, BMI2, Lzcnt]].concat(),
            Broadwell => Haswell.features(),
            Skylake => Broadwell.features(),
            Cannonlake => Skylake.features(),
            Icelake => Cannonlake.features(),
            Znver1 => vec![SSE3, SSSE3, SSE41, SSE42, Popcnt, AVX, BMI1, BMI2, Lzcnt],
        }
    }
}

/// Individual CPU features that may be used during codegen.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SpecificFeature {
    SSE3,
    SSSE3,
    SSE41,
    SSE42,
    Popcnt,
    AVX,
    BMI1,
    BMI2,
    Lzcnt,
}

/// An x86-specific configuration of CPU features that affect code generation.
#[derive(Debug, Clone)]
pub struct CpuFeatures {
    /// Base CPU profile to use
    cpu: TargetCpu,
    /// Specific CPU features to add or remove from the profile
    specific_features: HashMap<SpecificFeature, bool>,
}

fn detect_features(features: &mut ModuleFeatures) {
    let cpuid = CpuId::new();

    if let Some(info) = cpuid.get_feature_info() {
        features.sse3 = info.has_sse3();
        features.ssse3 = info.has_ssse3();
        features.sse41 = info.has_sse41();
        features.sse42 = info.has_sse42();
        features.avx = info.has_avx();
        features.popcnt = info.has_popcnt();
    }

    if let Some(info) = cpuid.get_extended_feature_info() {
        features.bmi1 = info.has_bmi1();
        features.bmi2 = info.has_bmi2();
    }

    if let Some(info) = cpuid.get_extended_function_info() {
        features.lzcnt = info.has_lzcnt();
    }
}

impl From<&CpuFeatures> for ModuleFeatures {
    fn from(cpu_features: &CpuFeatures) -> ModuleFeatures {
        let mut module_features = ModuleFeatures::none();

        let mut feature_set: HashSet<SpecificFeature> = HashSet::new();

        if let TargetCpu::Native = cpu_features.cpu {
            // If the target is `Native`, start with the current set of cpu features..
            detect_features(&mut module_features);
        } else {
            // otherwise, start with the target cpu's default feature set
            feature_set = cpu_features.cpu.features().into_iter().collect();
        }

        for (feature, enabled) in cpu_features.specific_features.iter() {
            if *enabled {
                feature_set.insert(*feature);
            } else {
                feature_set.remove(feature);
            }
        }

        for feature in feature_set {
            use SpecificFeature::*;
            match feature {
                SSE3 => {
                    module_features.sse3 = true;
                }
                SSSE3 => {
                    module_features.ssse3 = true;
                }
                SSE41 => {
                    module_features.sse41 = true;
                }
                SSE42 => {
                    module_features.sse42 = true;
                }
                AVX => {
                    module_features.avx = true;
                }
                BMI1 => {
                    module_features.bmi1 = true;
                }
                BMI2 => {
                    module_features.bmi2 = true;
                }
                Popcnt => {
                    module_features.popcnt = true;
                }
                Lzcnt => {
                    module_features.lzcnt = true;
                }
            }
        }
        module_features
    }
}

impl Default for CpuFeatures {
    fn default() -> Self {
        Self::detect_cpuid()
    }
}

impl CpuFeatures {
    pub fn new(cpu: TargetCpu, specific_features: HashMap<SpecificFeature, bool>) -> Self {
        Self {
            cpu,
            specific_features,
        }
    }

    /// Return a `CpuFeatures` that uses the CPUID instruction to determine which features to enable.
    pub fn detect_cpuid() -> Self {
        CpuFeatures {
            cpu: TargetCpu::Native,
            specific_features: HashMap::new(),
        }
    }

    /// Return a `CpuFeatures` with no optional features enabled.
    pub fn baseline() -> Self {
        CpuFeatures {
            cpu: TargetCpu::Baseline,
            specific_features: HashMap::new(),
        }
    }

    pub fn set(&mut self, sf: SpecificFeature, enabled: bool) {
        self.specific_features.insert(sf, enabled);
    }

    /// Return a `cranelift_codegen::isa::Builder` configured with these CPU features.

    pub fn isa_builder(&self, target: Triple) -> Result<isa::Builder, Error> {
        use SpecificFeature::*;
        use TargetCpu::*;

        let mut isa_builder = if let Native = self.cpu {
            cranelift_native::builder().map_err(|_| {
                Error::Unsupported("host machine is not a supported target".to_string())
            })
        } else {
            isa::lookup(target).map_err(Error::UnsupportedIsa)
        }?;

        let mut specific_features = self.specific_features.clone();

        // add any features from the CPU profile if they are not already individually specified
        for cpu_feature in self.cpu.features() {
            specific_features.entry(cpu_feature).or_insert(true);
        }

        for (feature, enabled) in specific_features.into_iter() {
            let enabled = if enabled { "true" } else { "false" };
            match feature {
                SSE3 => isa_builder.set("has_sse3", enabled).unwrap(),
                SSSE3 => isa_builder.set("has_ssse3", enabled).unwrap(),
                SSE41 => isa_builder.set("has_sse41", enabled).unwrap(),
                SSE42 => isa_builder.set("has_sse42", enabled).unwrap(),
                Popcnt => isa_builder.set("has_popcnt", enabled).unwrap(),
                AVX => isa_builder.set("has_avx", enabled).unwrap(),
                BMI1 => isa_builder.set("has_bmi1", enabled).unwrap(),
                BMI2 => isa_builder.set("has_bmi2", enabled).unwrap(),
                Lzcnt => isa_builder.set("has_lzcnt", enabled).unwrap(),
            }
        }

        Ok(isa_builder)
    }
}
