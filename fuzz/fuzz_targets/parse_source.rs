#![no_main]
use libfuzzer_sys::fuzz_target;

// Fuzz the Prism parser with arbitrary input treated as Ruby source.
// This catches panics from unexpected parse results, not just string handling.
fuzz_target!(|data: &[u8]| {
    let _ = nitrocop::parse::parse_source(data);
});
