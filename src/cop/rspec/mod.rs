pub mod align_left_let_brace;
pub mod align_right_let_brace;
pub mod any_instance;
pub mod around_block;
pub mod be;
pub mod be_empty;
pub mod be_eq;
pub mod be_eql;
pub mod be_nil;
pub mod before_after_all;
pub mod change_by_zero;
pub mod class_check;
pub mod contain_exactly;
pub mod context_method;
pub mod context_wording;
pub mod describe_class;
pub mod describe_method;
pub mod describe_symbol;
pub mod described_class;
pub mod described_class_module_wrapping;
pub mod dialect;
pub mod duplicated_metadata;
pub mod empty_example_group;
pub mod empty_hook;
pub mod empty_line_after_example;
pub mod empty_line_after_example_group;
pub mod empty_line_after_final_let;
pub mod empty_line_after_hook;
pub mod empty_line_after_subject;
pub mod empty_metadata;
pub mod empty_output;
pub mod eq;
pub mod example_length;
pub mod example_without_description;
pub mod example_wording;
pub mod excessive_docstring_spacing;
pub mod expect_actual;
pub mod expect_change;
pub mod expect_in_hook;
pub mod expect_in_let;
pub mod expect_output;
pub mod focus;
pub mod hook_argument;
pub mod hooks_before_examples;
pub mod identical_equality_assertion;
pub mod implicit_block_expectation;
pub mod implicit_expect;
pub mod implicit_subject;
pub mod include_examples;
pub mod indexed_let;
pub mod instance_spy;
pub mod instance_variable;
pub mod is_expected_specify;
pub mod it_behaves_like;
pub mod iterated_expectation;
pub mod leading_subject;
pub mod leaky_constant_declaration;
pub mod leaky_local_variable;
pub mod let_before_examples;
pub mod let_setup;
pub mod match_array;
pub mod message_chain;
pub mod message_expectation;
pub mod message_spies;
pub mod metadata_style;
pub mod missing_example_group_argument;
pub mod missing_expectation_target_method;
pub mod multiple_describes;
pub mod multiple_expectations;
pub mod multiple_memoized_helpers;
pub mod multiple_subjects;
pub mod named_subject;
pub mod nested_groups;
pub mod no_expectation_example;
pub mod not_to_not;
pub mod output;
pub mod overwriting_setup;
pub mod pending;
pub mod pending_without_reason;
pub mod predicate_matcher;
pub mod receive_counts;
pub mod receive_messages;
pub mod receive_never;
pub mod redundant_around;
pub mod redundant_predicate_matcher;
pub mod remove_const;
pub mod repeated_description;
pub mod repeated_example;
pub mod repeated_example_group_body;
pub mod repeated_example_group_description;
pub mod repeated_include_example;
pub mod repeated_subject_call;
pub mod return_from_stub;
pub mod scattered_let;
pub mod scattered_setup;
pub mod shared_context;
pub mod shared_examples;
pub mod single_argument_message_chain;
pub mod skip_block_inside_example;
pub mod sort_metadata;
pub mod spec_file_path_format;
pub mod spec_file_path_suffix;
pub mod stubbed_mock;
pub mod subject_declaration;
pub mod subject_stub;
pub mod undescriptive_literals_description;
pub mod unspecified_exception;
pub mod variable_definition;
pub mod variable_name;
pub mod verified_double_reference;
pub mod verified_doubles;
pub mod void_expect;
pub mod yield_cop;

use super::registry::CopRegistry;

pub fn register_all(registry: &mut CopRegistry) {
    registry.register(Box::new(align_left_let_brace::AlignLeftLetBrace));
    registry.register(Box::new(align_right_let_brace::AlignRightLetBrace));
    registry.register(Box::new(any_instance::AnyInstance));
    registry.register(Box::new(around_block::AroundBlock));
    registry.register(Box::new(be::Be));
    registry.register(Box::new(be_empty::BeEmpty));
    registry.register(Box::new(be_eq::BeEq));
    registry.register(Box::new(be_eql::BeEql));
    registry.register(Box::new(be_nil::BeNil));
    registry.register(Box::new(before_after_all::BeforeAfterAll));
    registry.register(Box::new(change_by_zero::ChangeByZero));
    registry.register(Box::new(class_check::ClassCheck));
    registry.register(Box::new(contain_exactly::ContainExactly));
    registry.register(Box::new(context_method::ContextMethod));
    registry.register(Box::new(context_wording::ContextWording));
    registry.register(Box::new(describe_class::DescribeClass));
    registry.register(Box::new(describe_method::DescribeMethod));
    registry.register(Box::new(describe_symbol::DescribeSymbol));
    registry.register(Box::new(described_class::DescribedClass));
    registry.register(Box::new(
        described_class_module_wrapping::DescribedClassModuleWrapping,
    ));
    registry.register(Box::new(dialect::Dialect));
    registry.register(Box::new(duplicated_metadata::DuplicatedMetadata));
    registry.register(Box::new(empty_example_group::EmptyExampleGroup));
    registry.register(Box::new(empty_hook::EmptyHook));
    registry.register(Box::new(empty_line_after_example::EmptyLineAfterExample));
    registry.register(Box::new(
        empty_line_after_example_group::EmptyLineAfterExampleGroup,
    ));
    registry.register(Box::new(empty_line_after_final_let::EmptyLineAfterFinalLet));
    registry.register(Box::new(empty_line_after_hook::EmptyLineAfterHook));
    registry.register(Box::new(empty_line_after_subject::EmptyLineAfterSubject));
    registry.register(Box::new(empty_metadata::EmptyMetadata));
    registry.register(Box::new(empty_output::EmptyOutput));
    registry.register(Box::new(eq::Eq));
    registry.register(Box::new(example_length::ExampleLength));
    registry.register(Box::new(
        example_without_description::ExampleWithoutDescription,
    ));
    registry.register(Box::new(example_wording::ExampleWording));
    registry.register(Box::new(
        excessive_docstring_spacing::ExcessiveDocstringSpacing,
    ));
    registry.register(Box::new(expect_actual::ExpectActual));
    registry.register(Box::new(expect_change::ExpectChange));
    registry.register(Box::new(expect_in_hook::ExpectInHook));
    registry.register(Box::new(expect_in_let::ExpectInLet));
    registry.register(Box::new(expect_output::ExpectOutput));
    registry.register(Box::new(focus::Focus));
    registry.register(Box::new(hook_argument::HookArgument));
    registry.register(Box::new(hooks_before_examples::HooksBeforeExamples));
    registry.register(Box::new(
        identical_equality_assertion::IdenticalEqualityAssertion,
    ));
    registry.register(Box::new(
        implicit_block_expectation::ImplicitBlockExpectation,
    ));
    registry.register(Box::new(implicit_expect::ImplicitExpect));
    registry.register(Box::new(implicit_subject::ImplicitSubject));
    registry.register(Box::new(include_examples::IncludeExamples));
    registry.register(Box::new(indexed_let::IndexedLet));
    registry.register(Box::new(instance_spy::InstanceSpy));
    registry.register(Box::new(instance_variable::InstanceVariable));
    registry.register(Box::new(is_expected_specify::IsExpectedSpecify));
    registry.register(Box::new(it_behaves_like::ItBehavesLike));
    registry.register(Box::new(iterated_expectation::IteratedExpectation));
    registry.register(Box::new(leading_subject::LeadingSubject));
    registry.register(Box::new(
        leaky_constant_declaration::LeakyConstantDeclaration,
    ));
    registry.register(Box::new(leaky_local_variable::LeakyLocalVariable));
    registry.register(Box::new(let_before_examples::LetBeforeExamples));
    registry.register(Box::new(let_setup::LetSetup));
    registry.register(Box::new(match_array::MatchArray));
    registry.register(Box::new(message_chain::MessageChain));
    registry.register(Box::new(message_expectation::MessageExpectation));
    registry.register(Box::new(message_spies::MessageSpies));
    registry.register(Box::new(metadata_style::MetadataStyle));
    registry.register(Box::new(
        missing_example_group_argument::MissingExampleGroupArgument,
    ));
    registry.register(Box::new(
        missing_expectation_target_method::MissingExpectationTargetMethod,
    ));
    registry.register(Box::new(multiple_describes::MultipleDescribes));
    registry.register(Box::new(multiple_expectations::MultipleExpectations));
    registry.register(Box::new(multiple_memoized_helpers::MultipleMemoizedHelpers));
    registry.register(Box::new(multiple_subjects::MultipleSubjects));
    registry.register(Box::new(named_subject::NamedSubject));
    registry.register(Box::new(nested_groups::NestedGroups));
    registry.register(Box::new(no_expectation_example::NoExpectationExample));
    registry.register(Box::new(not_to_not::NotToNot));
    registry.register(Box::new(output::Output));
    registry.register(Box::new(overwriting_setup::OverwritingSetup));
    registry.register(Box::new(pending::Pending));
    registry.register(Box::new(pending_without_reason::PendingWithoutReason));
    registry.register(Box::new(predicate_matcher::PredicateMatcher));
    registry.register(Box::new(receive_counts::ReceiveCounts));
    registry.register(Box::new(receive_messages::ReceiveMessages));
    registry.register(Box::new(receive_never::ReceiveNever));
    registry.register(Box::new(redundant_around::RedundantAround));
    registry.register(Box::new(
        redundant_predicate_matcher::RedundantPredicateMatcher,
    ));
    registry.register(Box::new(remove_const::RemoveConst));
    registry.register(Box::new(repeated_description::RepeatedDescription));
    registry.register(Box::new(repeated_example::RepeatedExample));
    registry.register(Box::new(
        repeated_example_group_body::RepeatedExampleGroupBody,
    ));
    registry.register(Box::new(
        repeated_example_group_description::RepeatedExampleGroupDescription,
    ));
    registry.register(Box::new(repeated_include_example::RepeatedIncludeExample));
    registry.register(Box::new(repeated_subject_call::RepeatedSubjectCall));
    registry.register(Box::new(return_from_stub::ReturnFromStub));
    registry.register(Box::new(scattered_let::ScatteredLet));
    registry.register(Box::new(scattered_setup::ScatteredSetup));
    registry.register(Box::new(shared_context::SharedContext));
    registry.register(Box::new(shared_examples::SharedExamples));
    registry.register(Box::new(
        single_argument_message_chain::SingleArgumentMessageChain,
    ));
    registry.register(Box::new(skip_block_inside_example::SkipBlockInsideExample));
    registry.register(Box::new(sort_metadata::SortMetadata));
    registry.register(Box::new(spec_file_path_format::SpecFilePathFormat));
    registry.register(Box::new(spec_file_path_suffix::SpecFilePathSuffix));
    registry.register(Box::new(stubbed_mock::StubbedMock));
    registry.register(Box::new(subject_declaration::SubjectDeclaration));
    registry.register(Box::new(subject_stub::SubjectStub));
    registry.register(Box::new(
        undescriptive_literals_description::UndescriptiveLiteralsDescription,
    ));
    registry.register(Box::new(unspecified_exception::UnspecifiedException));
    registry.register(Box::new(variable_definition::VariableDefinition));
    registry.register(Box::new(variable_name::VariableName));
    registry.register(Box::new(verified_double_reference::VerifiedDoubleReference));
    registry.register(Box::new(verified_doubles::VerifiedDoubles));
    registry.register(Box::new(void_expect::VoidExpect));
    registry.register(Box::new(yield_cop::Yield));
}
