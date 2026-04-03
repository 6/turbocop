use crate::cop::shared::node_type::{CALL_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Detects `.where()` and `.where.not()` calls with SQL strings that could be
/// replaced with hash syntax.
///
/// ## Investigation findings (2026-03-15)
///
/// Root cause of 198 FNs:
/// 1. **Array argument form not handled** — RuboCop accepts both `where("col = ?", val)`
///    and `where(["col = ?", val])`. nitrocop only handled the non-array form. The array
///    form wraps the SQL template and bind values in a single Array argument.
/// 2. **Receiverless `where` rejected** — `where('col = ?', val)` without an explicit
///    receiver (common in scopes, class methods, and blocks) was rejected by the
///    `receiver().is_none()` guard. RuboCop flags these. The `not` method still requires
///    a `where` receiver (`where.not(...)`).
///
/// ## Investigation findings (2026-03-31)
///
/// The remaining corpus false positives were real code bugs, not config artifacts.
/// nitrocop treated placeholder templates like `builder.where('1 = :zero')` and
/// `MINI_SQL.build(...).where('users.id = :user_id')` as offenses even though the
/// replacement value is supplied later, outside the `where` call. RuboCop only flags
/// `=` / `IN` placeholder forms when the same direct or array argument list includes
/// the bind value (`?`) or replacement hash (`:name`). This cop now requires that
/// inline replacement argument before reporting. Prism represents direct keyword
/// arguments like `where('name = :name', name: 'Gabe')` as `KeywordHashNode`,
/// while array-wrapped forms use `HashNode`, so both hash-like forms must be
/// accepted to avoid new false negatives.
pub struct WhereEquals;

impl Cop for WhereEquals {
    fn name(&self) -> &'static str {
        "Rails/WhereEquals"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let name = call.name().as_slice();
        if name != b"where" && name != b"not" {
            return;
        }

        // If `not`, check that receiver is a `where` call
        if name == b"not" {
            if let Some(recv) = call.receiver() {
                if let Some(recv_call) = recv.as_call_node() {
                    if recv_call.name().as_slice() != b"where" {
                        return;
                    }
                } else {
                    return;
                }
            } else {
                return;
            }
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // Extract the SQL template string and its inline replacement argument.
        // They can appear in two forms:
        // 1. Direct: where("col = ?", val)  — first arg is a StringNode
        // 2. Array:  where(["col = ?", val]) — first arg is an ArrayNode containing a StringNode
        let (template, has_bind_value, has_named_bind_hash) = if let Some(str_node) =
            arg_list[0].as_string_node()
        {
            (
                std::str::from_utf8(str_node.unescaped())
                    .unwrap_or("")
                    .to_string(),
                arg_list.len() > 1,
                arg_list
                    .get(1)
                    .map(|node| {
                        node.as_hash_node().is_some() || node.as_keyword_hash_node().is_some()
                    })
                    .unwrap_or(false),
            )
        } else if let Some(array_node) = arg_list[0].as_array_node() {
            let elements: Vec<_> = array_node.elements().iter().collect();
            if elements.is_empty() {
                return;
            }
            if let Some(str_node) = elements[0].as_string_node() {
                (
                    std::str::from_utf8(str_node.unescaped())
                        .unwrap_or("")
                        .to_string(),
                    elements.len() > 1,
                    elements
                        .get(1)
                        .map(|node| {
                            node.as_hash_node().is_some() || node.as_keyword_hash_node().is_some()
                        })
                        .unwrap_or(false),
                )
            } else {
                return;
            }
        } else {
            return;
        };

        // Check patterns:
        // column = ?
        // column IS NULL
        // column IN (?)
        let eq_anon = regex::Regex::new(r"^[\w.]+\s+=\s+\?$").unwrap();
        let in_anon = regex::Regex::new(r"(?i)^[\w.]+\s+IN\s+\(\?\)$").unwrap();
        let is_null = regex::Regex::new(r"(?i)^[\w.]+\s+IS\s+NULL$").unwrap();
        let eq_named = regex::Regex::new(r"^[\w.]+\s+=\s+:\w+$").unwrap();
        let in_named = regex::Regex::new(r"(?i)^[\w.]+\s+IN\s+\(:\w+\)$").unwrap();
        let matches_bound_placeholder =
            (eq_anon.is_match(&template) || in_anon.is_match(&template)) && has_bind_value;
        let matches_named_placeholder =
            (eq_named.is_match(&template) || in_named.is_match(&template)) && has_named_bind_hash;

        let is_simple_sql =
            matches_bound_placeholder || is_null.is_match(&template) || matches_named_placeholder;

        if !is_simple_sql {
            return;
        }

        // Reject database-qualified columns (e.g., "database.table.column") — only
        // table.column (one dot) or plain column (zero dots) are replaceable.
        let column_part = template
            .split(|c: char| !c.is_alphanumeric() && c != '.' && c != '_')
            .next()
            .unwrap_or("");
        if column_part.chars().filter(|&c| c == '.').count() > 1 {
            return;
        }

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let method = std::str::from_utf8(name).unwrap_or("where");
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{method}(attribute: value)` instead of manually constructing SQL."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(WhereEquals, "cops/rails/where_equals");

    #[test]
    fn test_array_argument_form() {
        let cop = WhereEquals;
        let source = b"User.where(['name = ?', 'Gabe'])\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(diags.len(), 1, "should detect array argument form");
    }

    #[test]
    fn test_array_is_null_form() {
        let cop = WhereEquals;
        let source = b"User.where(['name IS NULL'])\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(diags.len(), 1, "should detect array IS NULL form");
    }

    #[test]
    fn test_array_named_placeholder() {
        let cop = WhereEquals;
        let source = b"User.where(['name = :name', { name: 'Gabe' }])\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(diags.len(), 1, "should detect array named placeholder form");
    }

    #[test]
    fn test_array_in_form() {
        let cop = WhereEquals;
        let source = b"User.where([\"name IN (?)\", ['john', 'jane']])\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(diags.len(), 1, "should detect array IN form");
    }

    #[test]
    fn test_array_namespaced_column() {
        let cop = WhereEquals;
        let source = b"Course.where(['enrollments.student_id = ?', student.id])\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(diags.len(), 1, "should detect array namespaced column form");
    }

    #[test]
    fn test_where_not_regular_form() {
        let cop = WhereEquals;
        let source = b"User.where.not('name = ?', 'Gabe')\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(diags.len(), 1, "should detect where.not form");
    }

    #[test]
    fn test_scope_where() {
        let cop = WhereEquals;
        let source = b"scope :active, -> { where('active = ?', true) }\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(diags.len(), 1, "should detect where inside scope lambda");
    }

    #[test]
    fn test_chained_where() {
        let cop = WhereEquals;
        let source = b"User.active.where('name = ?', 'Gabe')\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(diags.len(), 1, "should detect chained where");
    }
}
