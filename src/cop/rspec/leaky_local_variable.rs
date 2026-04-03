use std::cell::RefCell;
use std::ops::Range;

use crate::cop::variable_force::{self, Scope, VariableTable};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

// Thread-local storage for per-file range data. Within a rayon task, a single
// file is processed sequentially: check_source → VF engine →
// before_leaving_scope, so thread-local storage is safe and avoids the
// TOCTOU race that Mutex fields on the shared cop struct would cause.
thread_local! {
    static LEAKY_RANGES: RefCell<LeakyRanges> = const { RefCell::new(LeakyRanges::new()) };
}

struct LeakyRanges {
    example_scope_ranges: Vec<Range<usize>>,
    example_group_ranges: Vec<Range<usize>>,
    allowed_ref_ranges: Vec<Range<usize>>,
}

impl LeakyRanges {
    const fn new() -> Self {
        Self {
            example_scope_ranges: Vec::new(),
            example_group_ranges: Vec::new(),
            allowed_ref_ranges: Vec::new(),
        }
    }
}

/// Flags local variable assignments at the example-group level that are then
/// referenced inside examples, hooks, let, or subject blocks. Use `let` instead.
///
/// ## VF-based implementation (Option C hybrid)
///
/// `check_source` pre-computes byte offset ranges for three categories:
/// - `example_scope_ranges`: it/specify/before/after/around/let/subject blocks
/// - `example_group_ranges`: describe/context/shared_examples blocks
/// - `allowed_ref_ranges`: it/specify description args, includes first args
///
/// The VF `before_leaving_scope` hook iterates each variable's per-assignment
/// references. An assignment is flagged when:
/// 1. The assignment is NOT inside an example scope
/// 2. At least one reference IS inside an example scope
/// 3. That reference IS inside an example group (describe block)
/// 4. That reference is NOT in an allowed reference range
///
/// Per-assignment tracking from VF gives us precise dead-assignment filtering
/// for free: if an assignment is overwritten before any read, it has no
/// references and is never flagged.
pub struct LeakyLocalVariable;

impl LeakyLocalVariable {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LeakyLocalVariable {
    fn default() -> Self {
        Self
    }
}

impl Cop for LeakyLocalVariable {
    fn name(&self) -> &'static str {
        "RSpec/LeakyLocalVariable"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn default_include(&self) -> &'static [&'static str] {
        crate::cop::util::RSPEC_DEFAULT_INCLUDE
    }

    fn check_source(
        &self,
        _source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
        _diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut collector = RangeCollector {
            example_scope_ranges: Vec::new(),
            example_group_ranges: Vec::new(),
            allowed_ref_ranges: Vec::new(),
        };
        collector.visit(&parse_result.node());

        // Sort ranges by start offset for binary search in offset_in_ranges
        collector
            .example_scope_ranges
            .sort_unstable_by_key(|r| r.start);
        collector
            .example_group_ranges
            .sort_unstable_by_key(|r| r.start);
        collector
            .allowed_ref_ranges
            .sort_unstable_by_key(|r| r.start);

        LEAKY_RANGES.with(|cell| {
            let mut ranges = cell.borrow_mut();
            ranges.example_scope_ranges = collector.example_scope_ranges;
            ranges.example_group_ranges = collector.example_group_ranges;
            ranges.allowed_ref_ranges = collector.allowed_ref_ranges;
        });
    }

    fn as_variable_force_consumer(&self) -> Option<&dyn variable_force::VariableForceConsumer> {
        Some(self)
    }
}

impl variable_force::VariableForceConsumer for LeakyLocalVariable {
    fn before_leaving_scope(
        &self,
        scope: &Scope,
        _variable_table: &VariableTable,
        source: &SourceFile,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Process all scope types — VF creates scopes for blocks, defs, modules, etc.
        // We rely on range checks rather than scope filtering.
        LEAKY_RANGES.with(|cell| {
            let ranges = cell.borrow();
            let es_ranges = &ranges.example_scope_ranges;
            let eg_ranges = &ranges.example_group_ranges;
            let ar_ranges = &ranges.allowed_ref_ranges;

            for variable in scope.variables.values() {
                for assignment in &variable.assignments {
                    // Skip assignments that are inside example scopes — they're local
                    if offset_in_ranges(assignment.node_offset, es_ranges) {
                        continue;
                    }

                    // Check if any reference to this assignment is:
                    // 1. Inside an example scope
                    // 2. Inside an example group (describe block)
                    // 3. NOT in an allowed reference range
                    let has_leaky_ref = assignment.references.iter().any(|ref_offset| {
                        offset_in_ranges(*ref_offset, es_ranges)
                            && offset_in_ranges(*ref_offset, eg_ranges)
                            && !offset_in_ranges(*ref_offset, ar_ranges)
                    });

                    if has_leaky_ref {
                        let (line, column) = source.offset_to_line_col(assignment.node_offset);
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Do not use local variables defined outside of examples inside of them."
                                .to_string(),
                        ));
                    }
                }
            }
        });
    }
}

/// Check if a byte offset falls within any of the sorted ranges.
/// Ranges may be nested, so we scan backwards through all candidates
/// whose start <= offset.
fn offset_in_ranges(offset: usize, ranges: &[Range<usize>]) -> bool {
    // Find the partition point: first range with start > offset
    let idx = ranges.partition_point(|r| r.start <= offset);
    // Scan backwards through all ranges that start at or before offset
    for i in (0..idx).rev() {
        if ranges[i].contains(&offset) {
            return true;
        }
        // Optimization: if we've gone past all ranges that could contain offset,
        // stop. Since ranges are sorted by start, and offset > range.start,
        // a range can only contain offset if range.end > offset.
        // But nested ranges can have arbitrarily large ends, so we must check all.
    }
    false
}

/// Prism visitor that collects byte offset ranges for example scopes,
/// example groups, and allowed reference zones.
struct RangeCollector {
    example_scope_ranges: Vec<Range<usize>>,
    example_group_ranges: Vec<Range<usize>>,
    allowed_ref_ranges: Vec<Range<usize>>,
}

impl RangeCollector {
    /// Register a call+block pair as an example scope.
    /// If `args_allowed` is true, also register the call's arguments
    /// as both an allowed reference range AND an example scope range
    /// (for `it "desc #{var}" do ... end` — the description args are
    /// allowed AND part of the example scope per RuboCop's `example_method?`).
    fn register_example_scope(
        &mut self,
        call: &ruby_prism::CallNode<'_>,
        block: &ruby_prism::BlockNode<'_>,
        args_allowed: bool,
    ) {
        // The example scope covers the entire block
        let block_loc = block.location();
        self.example_scope_ranges
            .push(block_loc.start_offset()..block_loc.end_offset());

        if args_allowed {
            // Arguments of the call (e.g., it "description", skip: message do ... end)
            // are both allowed reference zones AND part of the example scope
            if let Some(args) = call.arguments() {
                let args_loc = args.location();
                self.allowed_ref_ranges
                    .push(args_loc.start_offset()..args_loc.end_offset());
                self.example_scope_ranges
                    .push(args_loc.start_offset()..args_loc.end_offset());
            }
        }
    }

    /// Register an includes call (it_behaves_like, include_examples, etc.).
    /// The first arg is an allowed reference (the shared example name).
    /// Non-first args that are interpolated strings/symbols (dstr/dsym) are also allowed.
    /// The entire call is an example scope.
    fn register_includes_call(&mut self, call: &ruby_prism::CallNode<'_>) {
        // The whole call (including any block) is an example scope
        if let Some(block_node) = call.block() {
            if let Some(block) = block_node.as_block_node() {
                let block_loc = block.location();
                self.example_scope_ranges
                    .push(block_loc.start_offset()..block_loc.end_offset());
            }
        }

        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();

            // Entire args range is example scope
            let args_loc = args.location();
            self.example_scope_ranges
                .push(args_loc.start_offset()..args_loc.end_offset());

            // First arg is always allowed (shared example name)
            if let Some(first) = arg_list.first() {
                let loc = first.location();
                self.allowed_ref_ranges
                    .push(loc.start_offset()..loc.end_offset());
            }

            // Non-first args that are interpolated strings/symbols are allowed
            // (RuboCop's `allowed_includes_arguments?` allows dstr/dsym)
            for arg in arg_list.iter().skip(1) {
                if arg.as_interpolated_string_node().is_some()
                    || arg.as_interpolated_symbol_node().is_some()
                {
                    let loc = arg.location();
                    self.allowed_ref_ranges
                        .push(loc.start_offset()..loc.end_offset());
                }
            }
        }
    }
}

impl<'pr> Visit<'pr> for RangeCollector {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();

        // Example group methods work with or without receiver (describe, RSpec.describe, etc.)
        if is_example_group_method(method_name) {
            if let Some(block_node) = node.block() {
                if let Some(block) = block_node.as_block_node() {
                    let block_loc = block.location();
                    self.example_group_ranges
                        .push(block_loc.start_offset()..block_loc.end_offset());
                }
            }
        }

        // Only handle receiverless calls for example/hook/let/subject/includes
        if node.receiver().is_none() {
            if is_example_method(method_name) {
                // it/specify/example/scenario/its — example scope with allowed args
                if let Some(block_node) = node.block() {
                    if let Some(block) = block_node.as_block_node() {
                        self.register_example_scope(node, &block, true);
                    }
                } else {
                    // Blockless example: `it "description"` — args are both example scope and allowed
                    if let Some(args) = node.arguments() {
                        let args_loc = args.location();
                        self.example_scope_ranges
                            .push(args_loc.start_offset()..args_loc.end_offset());
                        self.allowed_ref_ranges
                            .push(args_loc.start_offset()..args_loc.end_offset());
                    }
                }
            } else if is_hook_method(method_name) {
                // before/after/around — example scope, no allowed args
                if let Some(block_node) = node.block() {
                    if let Some(block) = block_node.as_block_node() {
                        self.register_example_scope(node, &block, false);
                    }
                }
            } else if is_let_or_subject(method_name) {
                // let/let!/subject/subject! — example scope
                // Arguments (the let name like `:foo` or `html_options`) are always
                // registered as example scope, since `let(var)` reads the variable
                // in example-scope context.
                if let Some(args) = node.arguments() {
                    let args_loc = args.location();
                    self.example_scope_ranges
                        .push(args_loc.start_offset()..args_loc.end_offset());
                }
                if let Some(block_node) = node.block() {
                    if let Some(block) = block_node.as_block_node() {
                        self.register_example_scope(node, &block, false);
                    } else if block_node.as_block_argument_node().is_some() {
                        // Block-pass: `let(:foo, &bar)` — &bar is example scope
                        let loc = block_node.location();
                        self.example_scope_ranges
                            .push(loc.start_offset()..loc.end_offset());
                    }
                }
            } else if is_includes_method(method_name) {
                // it_behaves_like/it_should_behave_like/include_examples/include_context
                self.register_includes_call(node);
            }
        }

        // Continue recursion — visit receiver, arguments, and block children
        ruby_prism::visit_call_node(self, node);
    }
}

/// Example group methods: describe, context, shared_examples, shared_context, etc.
fn is_example_group_method(name: &[u8]) -> bool {
    let s = std::str::from_utf8(name).unwrap_or("");
    crate::cop::util::RSPEC_EXAMPLE_GROUPS.contains(&s)
        || crate::cop::util::RSPEC_SHARED_GROUPS.contains(&s)
}

/// Example methods: it, specify, example, scenario, its, etc.
fn is_example_method(name: &[u8]) -> bool {
    let s = std::str::from_utf8(name).unwrap_or("");
    crate::cop::util::RSPEC_EXAMPLES.contains(&s)
}

/// Hook methods: before, after, around, etc.
fn is_hook_method(name: &[u8]) -> bool {
    let s = std::str::from_utf8(name).unwrap_or("");
    crate::cop::util::RSPEC_HOOKS.contains(&s)
}

/// Let/subject methods
fn is_let_or_subject(name: &[u8]) -> bool {
    matches!(name, b"let" | b"let!" | b"subject" | b"subject!")
}

/// Includes methods: it_behaves_like, it_should_behave_like, include_examples, include_context
fn is_includes_method(name: &[u8]) -> bool {
    matches!(
        name,
        b"it_behaves_like" | b"it_should_behave_like" | b"include_examples" | b"include_context"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(LeakyLocalVariable::new(), "cops/rspec/leaky_local_variable");

    #[test]
    fn test_no_fp_iterator_var_only_in_description() {
        let source = br#"describe SomeClass do
  %w(d i).each do |f|
    format = "%" + f

    it "supports integer formats using #{format}" do
      ("%#{f}" % 10).should == "10"
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert!(
            diags.is_empty(),
            "Expected no offense for var used only in it description, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_no_fp_rswag_dsl_block() {
        let source = br#"describe SomeClass do
  path "/api/resource" do
    post "Create resource" do
      expected_schema = load_schema("create_request")
      parameter name: :params, in: :body, schema: expected_schema

      response "200", "success" do
        expected_schema = load_schema("create_response")
        schema expected_schema

        xit
      end
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert!(
            diags.is_empty(),
            "Expected no offense for rswag DSL block, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_no_fp_factory_bot() {
        let source = br#"FactoryBot.define :foo do
  bar = 123

  after(:create) do |foo|
    foo.update(bar: bar)
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert!(
            diags.is_empty(),
            "Expected no offense outside describe block, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_no_fp_capybara_spec_helper() {
        let source = br#"Capybara::SpecHelper.spec '#ancestor' do
  before do
    @session.visit('/with_html')
  end

  it 'should find the element' do
    el = @session.find(:css, '#child')
    expect(el.ancestor('//p')).to have_text('Lorem ipsum')
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert!(
            diags.is_empty(),
            "Expected no offense in Capybara::SpecHelper.spec, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_no_fp_var_used_in_describe_argument() {
        let source = br#"RSpec.describe(SomeClass) do
  result = described_class

  describe result::Success do
    it "works" do
      expect(true).to be true
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert!(
            diags.is_empty(),
            "Expected no offense for var used only in describe arg, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_no_fp_var_not_referenced_in_example() {
        let source = br#"describe SomeClass do
  user = create(:user)
  user.flag!

  it 'does something' do
    expect(foo).to eq(bar)
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert!(
            diags.is_empty(),
            "Expected no offense when var not used in example, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_no_fp_reassigned_before_read_in_example() {
        let source = br#"describe SomeClass do
  user = create(:user)

  it 'updates the user' do
    user = create(:user)
    expect { user.update(admin: true) }.to change(user, :updated_at)
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert!(
            diags.is_empty(),
            "Expected no offense when var reassigned before read, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_no_fp_it_description_only() {
        let source = br#"describe SomeClass do
  description = "updates the user"
  it description do
    expect { user.update(admin: true) }.to change(user, :updated_at)
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert!(
            diags.is_empty(),
            "Expected no offense when var used only as it description, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_no_fp_nil_init_reassigned_in_before() {
        let source = br#"describe SomeClass do
  result = nil

  before :each do
    result = compute_something()
  end

  it 'checks the result' do
    expect(result).to eq(42)
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert_eq!(
            diags.len(),
            0,
            "Expected 0 offenses (nil init reassigned in before hook), got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_no_fp_shared_context_vars() {
        let source_with_describe = br#"sc_extra = "file level"
RSpec.shared_context "test setup" do
  sc_opts = { timeout: 30 }
  before { setup(sc_opts) }
end
describe SomeClass do
  it "test" do
    expect(true).to be true
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source_with_describe);
        let offenses_at_line1: Vec<_> = diags.iter().filter(|d| d.location.line == 1).collect();
        assert!(
            offenses_at_line1.is_empty(),
            "sc_extra at file level should not be flagged (not used in examples)"
        );
    }

    #[test]
    fn test_no_fp_fastlane_file_level_nil_before_hook_reassign() {
        let source = br#"test_ui = nil
generator = nil

describe SomeClass do
  describe '#generate' do
    before(:each) do
      unless initialized
        test_ui = PluginGeneratorUI.new
        generator = PluginGenerator.new(ui: test_ui)
      end
    end

    after(:all) do
      test_ui = nil
      generator = nil
    end

    it 'generates plugin' do
      expect(generator).not_to be_nil
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert_eq!(
            diags.len(),
            0,
            "Expected 0 offenses (file-level nil vars reassigned in hooks), got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fp_devsec_control_block() {
        let source = br#"control 'sysctl-33' do
  flags = parse_config_file('/proc/cpuinfo').flags
  flags ||= ''
  flags = flags.split(' ')

  describe '/proc/cpuinfo' do
    it 'Flags should include NX' do
      expect(flags).to include('nx')
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense (last unconditional assignment only), got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
        assert_eq!(diags[0].location.line, 4, "Offense should be on line 4");
    }

    #[test]
    fn test_fn_nil_initialized_var_read_in_hook_predicate_before_write() {
        let source = br#"describe 'Puppet Ruby Generator' do
  context 'when generating static code' do
    module_def = nil

    before(:each) do
      if module_def.nil?
        module_def = build_module
      end
    end

    it 'keeps the generated module' do
      expect(module_def).not_to be_nil
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for nil init read in hook predicate, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
        assert_eq!(diags[0].location.line, 3, "Offense should be on line 3");
    }

    #[test]
    fn test_fn_def_body_vars_leak_into_describe() {
        let source = br#"def static_provider_resolution(opts = {})
  action         = opts[:action]
  provider_class = opts[:provider]
  resource_class = opts[:resource]

  describe resource_class, "static provider" do
    let(:node) do
      node = Object.new
      node
    end

    it "resolves the provider" do
      expect(provider_class).to eq(action)
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert!(
            diags.len() >= 2,
            "Expected at least 2 offenses for def body vars, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fn_module_body_vars_leak_into_describe() {
        let source = br#"module SamlIdp
  metadata_1 = <<-eos
<md:EntityDescriptor></md:EntityDescriptor>
  eos

  RSpec.describe 'incoming metadata' do
    it 'parses the metadata' do
      expect(metadata_1).to include('EntityDescriptor')
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for module body var leak, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
        assert_eq!(diags[0].location.line, 2, "Offense should be on line 2");
    }

    #[test]
    fn test_fn_each_block_param_reassignment() {
        let source = br#"describe SomeClass do
  items.each do |k|
    k = k.to_s

    it "includes the '#{k}' group" do
      expect(data[k]).to eq(subject.send(k))
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for block param reassignment, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fn_var_used_in_if_condition_with_let() {
        let source = br#"describe SomeClass do
  specs.each do |spec|
    context spec['name'] do
      if error = spec['error']
        let(:expected_error) { error }

        it 'fails' do
          expect { run }.to raise_error(expected_error)
        end
      end
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert!(
            diags.len() >= 1,
            "Expected at least 1 offense for var in if-condition with let, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fn_var_before_non_rspec_block_with_describe() {
        let source = br#"describe SomeClass do
  max_failures = 4
  failure_count = 0

  with_new_environment do
    spec = RSpec.describe "SomeTest" do
      it "test" do
        failure_count += 1
        if failure_count >= max_failures
          raise "too many failures"
        end
      end
    end

    spec.run
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert!(
            diags.len() >= 2,
            "Expected at least 2 offenses for vars before non-RSpec block with describe, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fn_var_used_in_let_name_argument() {
        let source = br#"RSpec.shared_examples 'a form field' do |field, html_options|
  html_options ||= :options

  include_context 'form', field

  context 'when class/id/data attributes are provided' do
    let(html_options) { { class: 'custom-field' } }

    it 'sets the attributes on the field' do
      expect(true).to eq(true)
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for let-name arg leak, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
        assert_eq!(diags[0].location.line, 2, "Offense should be on line 2");
    }

    #[test]
    fn test_fn_var_used_in_interpolated_xstring() {
        let source = br#"def which(cmd)
  cmd
end

insert_tee_log = '  2>&1 | tee -a vagrant.log ' if which('tee')

describe 'VM Life Cycle' do
  it 'starts Linux and Windows VM' do
    expect(`vagrant up  #{insert_tee_log}`).to include('tee')
  end

  it 'destroys Linux and Windows VM' do
    expect(`vagrant destroy --force  #{insert_tee_log}`).to include('Done removing resources')
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for interpolated xstring leak, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
        assert_eq!(diags[0].location.line, 5, "Offense should be on line 5");
    }

    #[test]
    fn test_fp_operator_write_kills_group_scope_value() {
        let source = br#"context 'when merged' do
  merged_config_methods = ::Leftovers.config.public_methods
  merged_config_methods -= ::Class.new.new.public_methods
  merged_config_methods -= %i{<<}

  it 'can build the voltron' do
    merged_config_methods.each { |method| ::Leftovers.config.send(method) }
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense (last -=), got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
        assert_eq!(diags[0].location.line, 4, "Offense on line 4 (last -=)");
    }

    #[test]
    fn test_fp_plus_equals_kills_group_scope_value() {
        let source = br#"RSpec.describe SomeClass do
  list_json_keys = %w[id user_id name]
  list_json_keys += %w[num_of_pages created_at]

  describe 'GET #index' do
    it 'renders index' do
      json = JSON.parse(response.body)
      list_json_keys.each do |k|
        expect(json.key?(k)).to eq(true)
      end
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense (+=), got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            diags[0].location.line, 3,
            "Offense should be on line 3 (+=)"
        );
    }

    #[test]
    fn test_fp_deep_write_before_read_in_it_block() {
        let source = br#"RSpec.describe 'Rails API completion' do
  filename = nil
  it 'provides Rails controller api' do
    map =
      rails_workspace do |root|
        filename = root.write_file 'test.rb', 'content'
      end
    expect(completion_at(filename, [1, 4], map)).to include('rescue_from')
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert_eq!(
            diags.len(),
            0,
            "Expected 0 offenses (deep write before read in it block), got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fn_def_body_var_in_else_branch_describe() {
        let source = br#"module EntitiesHelper
  def entity(name)
    path = "spec/entities/#{name}.yml"
    cassette = "entities-#{name}"
    if !File.exists?(path)
      File.write path, "data"
    else
      context name do
        let(:template) { File.read(path) }
        let(:fetched) { VCR.use_cassette(cassette) { get(name) } }
        it { is_expected.to eq template }
      end
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert!(
            diags.len() >= 2,
            "Expected at least 2 offenses for def body vars in else branch, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fn_let_block_argument() {
        let source = br#"RSpec.describe 'Weirdness' do
  bar = -> {}
  let(:foo, &bar)
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for let block argument, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
        assert_eq!(diags[0].location.line, 2, "Offense should be on line 2");
    }

    #[test]
    fn test_fn_begin_rescue_in_each_block() {
        let source = br#"describe 'REST API' do
  [1].each do |file|
    begin
      test_file = SomeClass.new(file)
    rescue StandardError => e
      next
    end
    context "test" do
      test_file.tests.each do |test|
        context test.description do
          before(:all) do
            test_file.setup
          end
        rescue StandardError => e
          raise e
        end
      end
    end
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for begin/rescue in each block, got {}: {:?}",
            diags.len(),
            diags
                .iter()
                .map(|d| format!("{}:{}", d.location.line, d.location.column))
                .collect::<Vec<_>>()
        );
        assert_eq!(diags[0].location.line, 4, "Offense should be on line 4");
    }

    #[test]
    fn test_fn_lambda_body_direct_assignment() {
        let source = br#"shared_examples_for 'streaming' do
  timing = 'ok'
  block = lambda do
    timing = 'not ok!'
  end
  it "gets response" do
    expect(timing).to eq('not ok!')
  end
end
"#;
        let diags = crate::testutil::run_cop_full(&LeakyLocalVariable::new(), source);
        let lines: Vec<_> = diags.iter().map(|d| d.location.line).collect();
        assert!(
            lines.contains(&4),
            "Expected offense on line 4 (timing = 'not ok!' in lambda), got: {:?}",
            lines
        );
    }
}
