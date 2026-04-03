use crate::cop::shared::node_type::{
    GLOBAL_VARIABLE_AND_WRITE_NODE, GLOBAL_VARIABLE_OPERATOR_WRITE_NODE,
    GLOBAL_VARIABLE_OR_WRITE_NODE, GLOBAL_VARIABLE_READ_NODE, GLOBAL_VARIABLE_TARGET_NODE,
    GLOBAL_VARIABLE_WRITE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/GlobalVars: flags uses of global variables not in the built-in list
/// or `AllowedVariables` config.
///
/// ## Investigation findings (2026-03)
///
/// ### FP root causes (68 total)
/// Missing built-in globals from RuboCop's BUILT_IN_VARS list:
/// `$DEFAULT_OUTPUT`, `$DEFAULT_INPUT`, `$IGNORECASE`, `$ARGV` (English aliases),
/// and `$CLASSPATH`, `$JRUBY_VERSION`, `$JRUBY_REVISION`, `$ENV_JAVA` (JRuby).
/// Top FP repos: jruby (27), byebug (14), ManageIQ (13), natalie (10).
/// These repos use JRuby built-ins and English aliases that we weren't treating
/// as built-in. Config resolution (`AllowedVariables`) was working correctly.
///
/// ### FN root causes (23 total)
/// `GlobalVariableTargetNode` (from `MultiWriteNode` / parallel assignment like
/// `$a, $b = 1, 2`) was not handled. Added to `interested_node_types`.
/// Top FN repos: natalie (8), eventmachine (6), rubychan (2).
///
/// ### AllowedVariables prefix handling
/// RuboCop converts `AllowedVariables` entries to symbols and compares against
/// `node.name` which includes the `$` prefix. Our implementation correctly
/// compares with `$` prefix. Additionally, we now also try matching without the
/// `$` prefix for robustness, since some configs may omit it.
pub struct GlobalVars;

// Built-in global variables and their English aliases, matching RuboCop's list.
// See: https://www.zenspider.com/ruby/quickref.html
const BUILTIN_GLOBALS: &[&[u8]] = &[
    b"$!",
    b"$@",
    b"$;",
    b"$,",
    b"$/",
    b"$\\",
    b"$.",
    b"$_",
    b"$~",
    b"$=",
    b"$*",
    b"$$",
    b"$?",
    b"$:",
    b"$\"",
    b"$<",
    b"$>",
    b"$0",
    b"$&",
    b"$`",
    b"$'",
    b"$+",
    b"$1",
    b"$2",
    b"$3",
    b"$4",
    b"$5",
    b"$6",
    b"$7",
    b"$8",
    b"$9",
    b"$PROGRAM_NAME",
    b"$VERBOSE",
    b"$DEBUG",
    b"$LOAD_PATH",
    b"$LOADED_FEATURES",
    b"$stdin",
    b"$stdout",
    b"$stderr",
    b"$FILENAME",
    b"$SAFE",
    b"$-a",
    b"$-d",
    b"$-i",
    b"$-l",
    b"$-p",
    b"$-v",
    b"$-w",
    b"$-0",
    b"$-F",
    b"$-I",
    b"$-K",
    b"$-W",
    b"$CHILD_STATUS",
    b"$ERROR_INFO",
    b"$ERROR_POSITION",
    b"$FIELD_SEPARATOR",
    b"$FS",
    b"$INPUT_LINE_NUMBER",
    b"$INPUT_RECORD_SEPARATOR",
    b"$LAST_MATCH_INFO",
    b"$LAST_PAREN_MATCH",
    b"$LAST_READ_LINE",
    b"$MATCH",
    b"$NR",
    b"$OFS",
    b"$ORS",
    b"$OUTPUT_FIELD_SEPARATOR",
    b"$OUTPUT_RECORD_SEPARATOR",
    b"$PID",
    b"$POSTMATCH",
    b"$PREMATCH",
    b"$PROCESS_ID",
    b"$RS",
    // English aliases missing from original list
    b"$DEFAULT_OUTPUT",
    b"$DEFAULT_INPUT",
    b"$IGNORECASE",
    b"$ARGV",
    // JRuby built-ins
    b"$CLASSPATH",
    b"$JRUBY_VERSION",
    b"$JRUBY_REVISION",
    b"$ENV_JAVA",
];

impl Cop for GlobalVars {
    fn name(&self) -> &'static str {
        "Style/GlobalVars"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            GLOBAL_VARIABLE_AND_WRITE_NODE,
            GLOBAL_VARIABLE_OPERATOR_WRITE_NODE,
            GLOBAL_VARIABLE_OR_WRITE_NODE,
            GLOBAL_VARIABLE_READ_NODE,
            GLOBAL_VARIABLE_TARGET_NODE,
            GLOBAL_VARIABLE_WRITE_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allowed = config.get_string_array("AllowedVariables");

        let (name, loc) = if let Some(gw) = node.as_global_variable_write_node() {
            let n = gw.name();
            (n.as_slice().to_vec(), gw.name_loc())
        } else if let Some(gr) = node.as_global_variable_read_node() {
            let n = gr.name();
            (n.as_slice().to_vec(), gr.location())
        } else if let Some(gow) = node.as_global_variable_operator_write_node() {
            let n = gow.name();
            (n.as_slice().to_vec(), gow.name_loc())
        } else if let Some(goaw) = node.as_global_variable_and_write_node() {
            let n = goaw.name();
            (n.as_slice().to_vec(), goaw.name_loc())
        } else if let Some(goow) = node.as_global_variable_or_write_node() {
            let n = goow.name();
            (n.as_slice().to_vec(), goow.name_loc())
        } else if let Some(gt) = node.as_global_variable_target_node() {
            let n = gt.name();
            (n.as_slice().to_vec(), gt.location())
        } else {
            return;
        };

        // Skip built-in globals
        if BUILTIN_GLOBALS.contains(&name.as_slice()) {
            return;
        }

        // Skip allowed variables — match with and without $ prefix for robustness.
        // RuboCop expects entries with $ prefix (e.g., "$CLASSPATH"), but some configs
        // may omit it, so we check both forms.
        let name_str = String::from_utf8_lossy(&name);
        let name_without_prefix = name_str.strip_prefix('$').unwrap_or(&name_str);
        if let Some(ref list) = allowed {
            if list.iter().any(|a| {
                let a = a.as_str();
                let a_without_prefix = a.strip_prefix('$').unwrap_or(a);
                a == name_str.as_ref() || a_without_prefix == name_without_prefix
            }) {
                return;
            }
        }

        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Do not introduce global variables.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(GlobalVars, "cops/style/global_vars");
}
