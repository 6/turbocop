use crate::cop::node_type::{
    GLOBAL_VARIABLE_AND_WRITE_NODE, GLOBAL_VARIABLE_OPERATOR_WRITE_NODE,
    GLOBAL_VARIABLE_OR_WRITE_NODE, GLOBAL_VARIABLE_READ_NODE, GLOBAL_VARIABLE_WRITE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct GlobalVars;

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
        } else {
            return;
        };

        // Skip built-in globals
        if BUILTIN_GLOBALS.iter().any(|&g| g == name.as_slice()) {
            return;
        }

        // Skip allowed variables
        let name_str = String::from_utf8_lossy(&name);
        if let Some(ref list) = allowed {
            if list.iter().any(|a| a.as_str() == name_str.as_ref()) {
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
