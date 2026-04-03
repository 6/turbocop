#![no_main]
use libfuzzer_sys::fuzz_target;

// Fuzz the rubocop directive cop-name normalizer.
// Exercises slash splitting, department merging, and suffix stripping.
fuzz_target!(|data: &str| {
    let _ = nitrocop::parse::directives::normalize_directive_cop_name(data);
});
