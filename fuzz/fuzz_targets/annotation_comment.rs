#![no_main]
use libfuzzer_sys::fuzz_target;

// Fuzz the comment annotation/directive checkers that do byte-offset slicing.
// These must never panic on arbitrary UTF-8 input.
fuzz_target!(|data: &str| {
    // Style/Documentation annotation checker
    let _ = nitrocop::cop::style::documentation::is_annotation_comment(data);

    // Style/DocumentationMethod annotation + directive checker (the original crash site)
    let _ =
        nitrocop::cop::style::documentation_method::is_annotation_or_directive_case_insensitive(
            data,
        );
});
