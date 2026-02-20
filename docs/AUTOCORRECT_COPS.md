# Autocorrectable Cops Reference

Complete catalog of every RuboCop cop that supports autocorrect, extracted from the vendor submodules. See [AUTOCORRECT_PLAN.md](AUTOCORRECT_PLAN.md) for the implementation plan.

## Summary

| Gem | Total Cops | Autocorrectable | % |
|-----|-----------|----------------|---|
| rubocop (core) | 593 | 458 | 77% |
| rubocop-performance | 52 | 45 | 87% |
| rubocop-rails | 148 | 101 | 68% |
| rubocop-rspec | 114 | 60 | 53% |
| **Total** | **907** | **664** | **73%** |

### Safety Classifications

| Category | Core | Performance | Rails | RSpec | Total |
|----------|------|-------------|-------|-------|-------|
| Safe (works with `-a`) | 391 | 36 | 73 | 44 | **544** |
| Unsafe (`SafeAutoCorrect: false`, needs `-A`) | 48 | 9 | 28 | 8 | **93** |
| Contextual (`AutoCorrect: contextual`) | 19 | 0 | 0 | 10 | **29** |
| **Total autocorrectable** | **458** | **45** | **101** | **60** | **664** |

Note: Some cops are both Unsafe and Contextual (e.g., `Lint/UselessOr`, `RSpec/EmptyExampleGroup`). They appear in both counts but are counted once in the total.

### Key

- **Safe** = works with `-a` flag (default behavior, no special config needed)
- **Unsafe** = has `SafeAutoCorrect: false` in config; requires `-A` flag
- **Contextual** = has `AutoCorrect: contextual`; skipped in LSP/editor mode

---

## RuboCop Core (458 autocorrectable / 593 total)

### Bundler (2 cops)

| Cop | Safety |
|-----|--------|
| Bundler/InsecureProtocolSource | Safe |
| Bundler/OrderedGems | Safe |

### Gemspec (4 cops)

| Cop | Safety |
|-----|--------|
| Gemspec/AddRuntimeDependency | Safe |
| Gemspec/DeprecatedAttributeAssignment | Safe |
| Gemspec/OrderedDependencies | Safe |
| Gemspec/RequireMfa | Safe |

### Layout (99 cops)

| Cop | Safety |
|-----|--------|
| Layout/AccessModifierIndentation | Safe |
| Layout/ArgumentAlignment | Safe |
| Layout/ArrayAlignment | Safe |
| Layout/AssignmentIndentation | Safe |
| Layout/BeginEndAlignment | Safe |
| Layout/BlockAlignment | Safe |
| Layout/BlockEndNewline | Safe |
| Layout/CaseIndentation | Safe |
| Layout/ClassStructure | Unsafe |
| Layout/ClosingHeredocIndentation | Safe |
| Layout/ClosingParenthesisIndentation | Safe |
| Layout/CommentIndentation | Safe |
| Layout/ConditionPosition | Safe |
| Layout/DefEndAlignment | Safe |
| Layout/DotPosition | Safe |
| Layout/ElseAlignment | Safe |
| Layout/EmptyComment | Contextual |
| Layout/EmptyLineAfterGuardClause | Safe |
| Layout/EmptyLineAfterMagicComment | Safe |
| Layout/EmptyLineAfterMultilineCondition | Safe |
| Layout/EmptyLineBetweenDefs | Safe |
| Layout/EmptyLines | Safe |
| Layout/EmptyLinesAfterModuleInclusion | Safe |
| Layout/EmptyLinesAroundAccessModifier | Safe |
| Layout/EmptyLinesAroundArguments | Safe |
| Layout/EmptyLinesAroundAttributeAccessor | Safe |
| Layout/EmptyLinesAroundBeginBody | Safe |
| Layout/EmptyLinesAroundBlockBody | Safe |
| Layout/EmptyLinesAroundClassBody | Safe |
| Layout/EmptyLinesAroundExceptionHandlingKeywords | Safe |
| Layout/EmptyLinesAroundMethodBody | Safe |
| Layout/EmptyLinesAroundModuleBody | Safe |
| Layout/EndAlignment | Safe |
| Layout/ExtraSpacing | Safe |
| Layout/FirstArgumentIndentation | Safe |
| Layout/FirstArrayElementIndentation | Safe |
| Layout/FirstArrayElementLineBreak | Safe |
| Layout/FirstHashElementIndentation | Safe |
| Layout/FirstHashElementLineBreak | Safe |
| Layout/FirstMethodArgumentLineBreak | Safe |
| Layout/FirstMethodParameterLineBreak | Safe |
| Layout/FirstParameterIndentation | Safe |
| Layout/HashAlignment | Safe |
| Layout/HeredocArgumentClosingParenthesis | Safe |
| Layout/HeredocIndentation | Safe |
| Layout/IndentationConsistency | Safe |
| Layout/IndentationStyle | Safe |
| Layout/IndentationWidth | Safe |
| Layout/InitialIndentation | Safe |
| Layout/LeadingCommentSpace | Safe |
| Layout/LeadingEmptyLines | Safe |
| Layout/LineContinuationLeadingSpace | Safe |
| Layout/LineContinuationSpacing | Safe |
| Layout/LineEndStringConcatenationIndentation | Safe |
| Layout/LineLength | Safe |
| Layout/MultilineArrayBraceLayout | Safe |
| Layout/MultilineArrayLineBreaks | Safe |
| Layout/MultilineAssignmentLayout | Safe |
| Layout/MultilineBlockLayout | Safe |
| Layout/MultilineHashBraceLayout | Safe |
| Layout/MultilineHashKeyLineBreaks | Safe |
| Layout/MultilineMethodArgumentLineBreaks | Safe |
| Layout/MultilineMethodCallBraceLayout | Safe |
| Layout/MultilineMethodCallIndentation | Safe |
| Layout/MultilineMethodDefinitionBraceLayout | Safe |
| Layout/MultilineMethodParameterLineBreaks | Safe |
| Layout/MultilineOperationIndentation | Safe |
| Layout/ParameterAlignment | Safe |
| Layout/RedundantLineBreak | Safe |
| Layout/RescueEnsureAlignment | Safe |
| Layout/SingleLineBlockChain | Safe |
| Layout/SpaceAfterColon | Safe |
| Layout/SpaceAfterComma | Safe |
| Layout/SpaceAfterMethodName | Safe |
| Layout/SpaceAfterNot | Safe |
| Layout/SpaceAfterSemicolon | Safe |
| Layout/SpaceAroundBlockParameters | Safe |
| Layout/SpaceAroundEqualsInParameterDefault | Safe |
| Layout/SpaceAroundKeyword | Safe |
| Layout/SpaceAroundMethodCallOperator | Safe |
| Layout/SpaceAroundOperators | Safe |
| Layout/SpaceBeforeBlockBraces | Safe |
| Layout/SpaceBeforeBrackets | Safe |
| Layout/SpaceBeforeComma | Safe |
| Layout/SpaceBeforeComment | Safe |
| Layout/SpaceBeforeFirstArg | Safe |
| Layout/SpaceBeforeSemicolon | Safe |
| Layout/SpaceInLambdaLiteral | Safe |
| Layout/SpaceInsideArrayLiteralBrackets | Safe |
| Layout/SpaceInsideArrayPercentLiteral | Safe |
| Layout/SpaceInsideBlockBraces | Safe |
| Layout/SpaceInsideHashLiteralBraces | Safe |
| Layout/SpaceInsideParens | Safe |
| Layout/SpaceInsidePercentLiteralDelimiters | Safe |
| Layout/SpaceInsideRangeLiteral | Safe |
| Layout/SpaceInsideReferenceBrackets | Safe |
| Layout/SpaceInsideStringInterpolation | Safe |
| Layout/TrailingEmptyLines | Safe |
| Layout/TrailingWhitespace | Safe |

### Lint (85 cops)

| Cop | Safety |
|-----|--------|
| Lint/AmbiguousBlockAssociation | Safe |
| Lint/AmbiguousOperator | Safe |
| Lint/AmbiguousOperatorPrecedence | Safe |
| Lint/AmbiguousRange | Unsafe |
| Lint/AmbiguousRegexpLiteral | Safe |
| Lint/ArrayLiteralInRegexp | Unsafe |
| Lint/AssignmentInCondition | Unsafe |
| Lint/BigDecimalNew | Safe |
| Lint/BooleanSymbol | Unsafe |
| Lint/ConstantOverwrittenInRescue | Safe |
| Lint/DeprecatedClassMethods | Safe |
| Lint/DeprecatedConstants | Safe |
| Lint/DeprecatedOpenSSLConstant | Safe |
| Lint/DisjunctiveAssignmentInConstructor | Safe |
| Lint/DuplicateMagicComment | Safe |
| Lint/DuplicateRegexpCharacterClassElement | Safe |
| Lint/DuplicateRequire | Unsafe |
| Lint/DuplicateSetElement | Safe |
| Lint/ElseLayout | Safe |
| Lint/EmptyConditionalBody | Contextual |
| Lint/EmptyEnsure | Contextual |
| Lint/EmptyInterpolation | Contextual |
| Lint/ErbNewArguments | Safe |
| Lint/HashNewWithKeywordArgumentsAsDefault | Safe |
| Lint/HeredocMethodCallPosition | Safe |
| Lint/IdentityComparison | Safe |
| Lint/ImplicitStringConcatenation | Safe |
| Lint/IncompatibleIoSelectWithFiberScheduler | Unsafe |
| Lint/InheritException | Unsafe |
| Lint/InterpolationCheck | Unsafe |
| Lint/LambdaWithoutLiteralBlock | Safe |
| Lint/LiteralAsCondition | Contextual |
| Lint/LiteralInInterpolation | Safe |
| Lint/Loop | Safe |
| Lint/MixedCaseRange | Unsafe |
| Lint/MultipleComparison | Safe |
| Lint/NonAtomicFileOperation | Unsafe |
| Lint/NonDeterministicRequireOrder | Safe |
| Lint/NumberConversion | Unsafe |
| Lint/NumericOperationWithConstantResult | Safe |
| Lint/OrAssignmentToConstant | Safe |
| Lint/OrderedMagicComments | Unsafe |
| Lint/ParenthesesAsGroupedExpression | Safe |
| Lint/PercentStringArray | Safe |
| Lint/PercentSymbolArray | Safe |
| Lint/RaiseException | Safe |
| Lint/RedundantCopDisableDirective | Safe |
| Lint/RedundantCopEnableDirective | Safe |
| Lint/RedundantDirGlobSort | Unsafe |
| Lint/RedundantRegexpQuantifiers | Safe |
| Lint/RedundantRequireStatement | Safe |
| Lint/RedundantSafeNavigation | Safe |
| Lint/RedundantSplatExpansion | Safe |
| Lint/RedundantStringCoercion | Safe |
| Lint/RedundantTypeConversion | Safe |
| Lint/RedundantWithIndex | Safe |
| Lint/RedundantWithObject | Safe |
| Lint/RegexpAsCondition | Safe |
| Lint/RequireRelativeSelfPath | Safe |
| Lint/RescueType | Safe |
| Lint/SafeNavigationChain | Safe |
| Lint/SafeNavigationConsistency | Safe |
| Lint/SafeNavigationWithEmpty | Safe |
| Lint/ScriptPermission | Safe |
| Lint/SendWithMixinArgument | Safe |
| Lint/SuppressedExceptionInNumberConversion | Unsafe |
| Lint/SymbolConversion | Safe |
| Lint/ToJSON | Safe |
| Lint/TopLevelReturnWithArgument | Safe |
| Lint/TrailingCommaInAttributeDeclaration | Contextual |
| Lint/TripleQuotes | Safe |
| Lint/UnescapedBracketInRegexp | Safe |
| Lint/UnifiedInteger | Safe |
| Lint/UnusedBlockArgument | Contextual |
| Lint/UnusedMethodArgument | Contextual |
| Lint/UriRegexp | Safe |
| Lint/UselessAccessModifier | Contextual |
| Lint/UselessAssignment | Contextual |
| Lint/UselessDefaultValueArgument | Safe |
| Lint/UselessMethodDefinition | Contextual |
| Lint/UselessNumericOperation | Safe |
| Lint/UselessOr | Unsafe + Contextual |
| Lint/UselessSetterCall | Safe |
| Lint/UselessTimes | Contextual |
| Lint/Void | Contextual |

### Migration (1 cop)

| Cop | Safety |
|-----|--------|
| Migration/DepartmentName | Safe |

### Naming (6 cops)

| Cop | Safety |
|-----|--------|
| Naming/BinaryOperatorParameterName | Safe |
| Naming/BlockForwarding | Safe |
| Naming/HeredocDelimiterCase | Safe |
| Naming/InclusiveLanguage | Safe |
| Naming/MemoizedInstanceVariableName | Safe |
| Naming/RescuedExceptionsVariableName | Safe |

### Security (3 cops)

| Cop | Safety |
|-----|--------|
| Security/IoMethods | Safe |
| Security/JSONLoad | Unsafe |
| Security/YAMLLoad | Unsafe |

### Style (258 cops)

| Cop | Safety |
|-----|--------|
| Style/AccessModifierDeclarations | Unsafe |
| Style/AccessorGrouping | Safe |
| Style/Alias | Safe |
| Style/AmbiguousEndlessMethodDefinition | Safe |
| Style/AndOr | Unsafe |
| Style/ArgumentsForwarding | Safe |
| Style/ArrayCoercion | Safe |
| Style/ArrayFirstLast | Safe |
| Style/ArrayIntersect | Safe |
| Style/ArrayIntersectWithSingleElement | Safe |
| Style/ArrayJoin | Safe |
| Style/Attr | Safe |
| Style/BarePercentLiterals | Safe |
| Style/BisectedAttrAccessor | Safe |
| Style/BitwisePredicate | Safe |
| Style/BlockComments | Safe |
| Style/BlockDelimiters | Safe |
| Style/CaseEquality | Safe |
| Style/CaseLikeIf | Safe |
| Style/CharacterLiteral | Safe |
| Style/ClassAndModuleChildren | Unsafe |
| Style/ClassCheck | Safe |
| Style/ClassEqualityComparison | Unsafe |
| Style/ClassMethods | Safe |
| Style/ClassMethodsDefinitions | Safe |
| Style/CollectionCompact | Safe |
| Style/CollectionMethods | Safe |
| Style/CollectionQuerying | Safe |
| Style/ColonMethodCall | Safe |
| Style/ColonMethodDefinition | Safe |
| Style/CombinableDefined | Safe |
| Style/CombinableLoops | Safe |
| Style/CommandLiteral | Safe |
| Style/CommentAnnotation | Safe |
| Style/CommentedKeyword | Unsafe |
| Style/ComparableBetween | Safe |
| Style/ComparableClamp | Safe |
| Style/ConcatArrayLiterals | Safe |
| Style/ConditionalAssignment | Safe |
| Style/Copyright | Safe |
| Style/DataInheritance | Unsafe |
| Style/DateTime | Unsafe |
| Style/DefWithParentheses | Safe |
| Style/DigChain | Safe |
| Style/Dir | Safe |
| Style/DirEmpty | Safe |
| Style/DisableCopsWithinSourceCodeDirective | Safe |
| Style/DoubleCopDisableDirective | Safe |
| Style/DoubleNegation | Unsafe |
| Style/EachForSimpleLoop | Safe |
| Style/EachWithObject | Safe |
| Style/EmptyBlockParameter | Safe |
| Style/EmptyCaseCondition | Safe |
| Style/EmptyClassDefinition | Safe |
| Style/EmptyElse | Contextual |
| Style/EmptyHeredoc | Contextual |
| Style/EmptyLambdaParameter | Safe |
| Style/EmptyLiteral | Safe |
| Style/EmptyMethod | Contextual |
| Style/EmptyStringInsideInterpolation | Safe |
| Style/Encoding | Safe |
| Style/EndBlock | Safe |
| Style/EndlessMethod | Safe |
| Style/EnvHome | Safe |
| Style/EvalWithLocation | Safe |
| Style/EvenOdd | Safe |
| Style/ExactRegexpMatch | Safe |
| Style/ExpandPathArguments | Safe |
| Style/ExplicitBlockArgument | Safe |
| Style/FetchEnvVar | Safe |
| Style/FileEmpty | Safe |
| Style/FileNull | Unsafe |
| Style/FileRead | Safe |
| Style/FileTouch | Unsafe |
| Style/FileWrite | Safe |
| Style/FloatDivision | Safe |
| Style/For | Unsafe |
| Style/FormatString | Safe |
| Style/FormatStringToken | Safe |
| Style/FrozenStringLiteralComment | Unsafe |
| Style/GlobalStdStream | Unsafe |
| Style/GuardClause | Safe |
| Style/HashAsLastArrayItem | Safe |
| Style/HashConversion | Unsafe |
| Style/HashEachMethods | Safe |
| Style/HashExcept | Safe |
| Style/HashFetchChain | Safe |
| Style/HashLookupMethod | Safe |
| Style/HashSlice | Safe |
| Style/HashSyntax | Safe |
| Style/HashTransformKeys | Safe |
| Style/HashTransformValues | Safe |
| Style/IdenticalConditionalBranches | Unsafe |
| Style/IfInsideElse | Safe |
| Style/IfUnlessModifier | Safe |
| Style/IfUnlessModifierOfIfUnless | Safe |
| Style/IfWithBooleanLiteralBranches | Unsafe |
| Style/IfWithSemicolon | Safe |
| Style/InPatternThen | Safe |
| Style/InfiniteLoop | Safe |
| Style/InverseMethods | Safe |
| Style/InvertibleUnlessCondition | Safe |
| Style/ItBlockParameter | Safe |
| Style/KeywordArgumentsMerging | Safe |
| Style/KeywordParametersOrder | Safe |
| Style/Lambda | Safe |
| Style/LambdaCall | Contextual |
| Style/LineEndConcatenation | Unsafe |
| Style/MagicCommentFormat | Safe |
| Style/MapCompactWithConditionalBlock | Safe |
| Style/MapIntoArray | Safe |
| Style/MapToHash | Safe |
| Style/MapToSet | Safe |
| Style/MethodCallWithArgsParentheses | Safe |
| Style/MethodCallWithoutArgsParentheses | Safe |
| Style/MethodDefParentheses | Safe |
| Style/MinMax | Safe |
| Style/MinMaxComparison | Safe |
| Style/MissingElse | Safe |
| Style/MixinGrouping | Safe |
| Style/ModuleFunction | Unsafe |
| Style/ModuleMemberExistenceCheck | Safe |
| Style/MultilineIfModifier | Safe |
| Style/MultilineIfThen | Safe |
| Style/MultilineInPatternThen | Safe |
| Style/MultilineMemoization | Safe |
| Style/MultilineMethodSignature | Safe |
| Style/MultilineTernaryOperator | Safe |
| Style/MultilineWhenThen | Safe |
| Style/MultipleComparison | Safe |
| Style/MutableConstant | Unsafe |
| Style/NegatedIf | Safe |
| Style/NegatedIfElseCondition | Safe |
| Style/NegatedUnless | Safe |
| Style/NegatedWhile | Safe |
| Style/NegativeArrayIndex | Safe |
| Style/NestedFileDirname | Safe |
| Style/NestedModifier | Safe |
| Style/NestedParenthesizedCalls | Safe |
| Style/NestedTernaryOperator | Safe |
| Style/Next | Safe |
| Style/NilComparison | Safe |
| Style/NilLambda | Safe |
| Style/NonNilCheck | Safe |
| Style/Not | Safe |
| Style/NumericLiteralPrefix | Safe |
| Style/NumericLiterals | Safe |
| Style/NumericPredicate | Safe |
| Style/ObjectThen | Safe |
| Style/OneLineConditional | Safe |
| Style/OperatorMethodCall | Safe |
| Style/OrAssignment | Safe |
| Style/ParallelAssignment | Safe |
| Style/ParenthesesAroundCondition | Safe |
| Style/PercentLiteralDelimiters | Safe |
| Style/PercentQLiterals | Safe |
| Style/PerlBackrefs | Safe |
| Style/PreferredHashMethods | Safe |
| Style/Proc | Safe |
| Style/QuotedSymbols | Safe |
| Style/RaiseArgs | Safe |
| Style/RandomWithOffset | Safe |
| Style/RedundantArgument | Safe |
| Style/RedundantArrayConstructor | Safe |
| Style/RedundantArrayFlatten | Safe |
| Style/RedundantAssignment | Safe |
| Style/RedundantBegin | Safe |
| Style/RedundantCapitalW | Safe |
| Style/RedundantCondition | Safe |
| Style/RedundantConditional | Safe |
| Style/RedundantConstantBase | Safe |
| Style/RedundantCurrentDirectoryInPath | Safe |
| Style/RedundantDoubleSplatHashBraces | Safe |
| Style/RedundantEach | Safe |
| Style/RedundantException | Safe |
| Style/RedundantFetchBlock | Safe |
| Style/RedundantFileExtensionInRequire | Safe |
| Style/RedundantFilterChain | Unsafe |
| Style/RedundantFormat | Unsafe |
| Style/RedundantFreeze | Safe |
| Style/RedundantHeredocDelimiterQuotes | Safe |
| Style/RedundantInitialize | Contextual |
| Style/RedundantInterpolation | Unsafe |
| Style/RedundantInterpolationUnfreeze | Safe |
| Style/RedundantLineContinuation | Safe |
| Style/RedundantParentheses | Safe |
| Style/RedundantPercentQ | Safe |
| Style/RedundantRegexpArgument | Safe |
| Style/RedundantRegexpCharacterClass | Safe |
| Style/RedundantRegexpConstructor | Safe |
| Style/RedundantRegexpEscape | Safe |
| Style/RedundantReturn | Safe |
| Style/RedundantSelf | Safe |
| Style/RedundantSelfAssignment | Safe |
| Style/RedundantSelfAssignmentBranch | Safe |
| Style/RedundantSort | Safe |
| Style/RedundantSortBy | Safe |
| Style/RedundantStringEscape | Safe |
| Style/RegexpLiteral | Safe |
| Style/RequireOrder | Unsafe |
| Style/RescueModifier | Safe |
| Style/RescueStandardError | Safe |
| Style/ReturnNil | Safe |
| Style/ReturnNilInPredicateMethodDefinition | Unsafe |
| Style/ReverseFind | Safe |
| Style/SafeNavigation | Unsafe |
| Style/Sample | Safe |
| Style/SelectByRegexp | Unsafe |
| Style/SelfAssignment | Safe |
| Style/Semicolon | Safe |
| Style/SendWithLiteralMethodName | Safe |
| Style/SignalException | Safe |
| Style/SingleArgumentDig | Safe |
| Style/SingleLineBlockParams | Safe |
| Style/SingleLineDoEndBlock | Safe |
| Style/SingleLineMethods | Safe |
| Style/SlicingWithRange | Safe |
| Style/SoleNestedConditional | Safe |
| Style/SpecialGlobalVars | Unsafe |
| Style/StabbyLambdaParentheses | Safe |
| Style/StaticClass | Safe |
| Style/StderrPuts | Safe |
| Style/StringChars | Safe |
| Style/StringConcatenation | Safe |
| Style/StringHashKeys | Safe |
| Style/StringLiterals | Safe |
| Style/StringLiteralsInInterpolation | Safe |
| Style/StringMethods | Safe |
| Style/Strip | Safe |
| Style/StructInheritance | Unsafe |
| Style/SuperArguments | Safe |
| Style/SuperWithArgsParentheses | Safe |
| Style/SwapValues | Unsafe |
| Style/SymbolArray | Safe |
| Style/SymbolLiteral | Safe |
| Style/SymbolProc | Safe |
| Style/TernaryParentheses | Safe |
| Style/TrailingBodyOnClass | Safe |
| Style/TrailingBodyOnMethodDefinition | Safe |
| Style/TrailingBodyOnModule | Safe |
| Style/TrailingCommaInArguments | Safe |
| Style/TrailingCommaInArrayLiteral | Safe |
| Style/TrailingCommaInBlockArgs | Safe |
| Style/TrailingCommaInHashLiteral | Safe |
| Style/TrailingMethodEndStatement | Safe |
| Style/TrailingUnderscoreVariable | Safe |
| Style/TrivialAccessors | Safe |
| Style/UnlessElse | Safe |
| Style/UnpackFirst | Safe |
| Style/VariableInterpolation | Safe |
| Style/WhenThen | Safe |
| Style/WhileUntilDo | Safe |
| Style/WhileUntilModifier | Safe |
| Style/WordArray | Safe |
| Style/YAMLFileRead | Safe |
| Style/YodaCondition | Safe |
| Style/YodaExpression | Safe |
| Style/ZeroLengthPredicate | Safe |

## RuboCop-Performance (45 autocorrectable / 52 total)

| Cop | Safety |
|-----|--------|
| Performance/AncestorsInclude | Safe |
| Performance/ArraySemiInfiniteRangeSlice | Safe |
| Performance/BigDecimalWithNumericArgument | Safe |
| Performance/BindCall | Safe |
| Performance/BlockGivenWithExplicitBlock | Safe |
| Performance/Caller | Safe |
| Performance/CaseWhenSplat | Unsafe |
| Performance/Casecmp | Safe |
| Performance/CompareWithBlock | Safe |
| Performance/ConcurrentMonotonicTime | Safe |
| Performance/ConstantRegexp | Safe |
| Performance/Count | Unsafe |
| Performance/DeletePrefix | Safe |
| Performance/DeleteSuffix | Safe |
| Performance/Detect | Unsafe |
| Performance/DoubleStartEndWith | Safe |
| Performance/EndWith | Unsafe |
| Performance/FlatMap | Safe |
| Performance/InefficientHashSearch | Safe |
| Performance/IoReadlines | Safe |
| Performance/MapCompact | Safe |
| Performance/RangeInclude | Safe |
| Performance/RedundantBlockCall | Safe |
| Performance/RedundantEqualityComparisonBlock | Safe |
| Performance/RedundantMatch | Safe |
| Performance/RedundantMerge | Safe |
| Performance/RedundantSortBlock | Safe |
| Performance/RedundantSplitRegexpArgument | Safe |
| Performance/RedundantStringChars | Safe |
| Performance/RegexpMatch | Safe |
| Performance/ReverseEach | Safe |
| Performance/ReverseFirst | Safe |
| Performance/Size | Safe |
| Performance/SortReverse | Safe |
| Performance/Squeeze | Safe |
| Performance/StartWith | Unsafe |
| Performance/StringBytesize | Safe |
| Performance/StringIdentifierArgument | Safe |
| Performance/StringInclude | Unsafe |
| Performance/StringReplacement | Safe |
| Performance/Sum | Unsafe |
| Performance/TimesMap | Unsafe |
| Performance/UnfreezeString | Unsafe |
| Performance/UriDefaultParser | Safe |
| Performance/ZipWithoutBlock | Safe |

## RuboCop-Rails (101 autocorrectable / 148 total)

| Cop | Safety |
|-----|--------|
| Rails/ActionControllerFlashBeforeRender | Unsafe |
| Rails/ActionControllerTestCase | Unsafe |
| Rails/ActionFilter | Safe |
| Rails/ActionOrder | Safe |
| Rails/ActiveRecordAliases | Unsafe |
| Rails/ActiveRecordCallbacksOrder | Safe |
| Rails/ActiveSupportAliases | Safe |
| Rails/ActiveSupportOnLoad | Unsafe |
| Rails/AddColumnIndex | Safe |
| Rails/ApplicationController | Unsafe |
| Rails/ApplicationJob | Unsafe |
| Rails/ApplicationMailer | Unsafe |
| Rails/ApplicationRecord | Unsafe |
| Rails/ArelStar | Unsafe |
| Rails/AssertNot | Safe |
| Rails/AttributeDefaultBlockValue | Safe |
| Rails/BelongsTo | Safe |
| Rails/Blank | Unsafe |
| Rails/CompactBlank | Safe |
| Rails/ContentTag | Safe |
| Rails/Date | Unsafe |
| Rails/Delegate | Safe |
| Rails/DelegateAllowBlank | Safe |
| Rails/DeprecatedActiveModelErrorsMethods | Safe |
| Rails/DotSeparatedKeys | Safe |
| Rails/DuplicateAssociation | Safe |
| Rails/DurationArithmetic | Safe |
| Rails/DynamicFindBy | Safe |
| Rails/EagerEvaluationLogMessage | Safe |
| Rails/EnumHash | Safe |
| Rails/EnumSyntax | Safe |
| Rails/EnvLocal | Safe |
| Rails/EnvironmentComparison | Safe |
| Rails/ExpandedDateRange | Safe |
| Rails/FilePath | Safe |
| Rails/FindBy | Safe |
| Rails/FindById | Safe |
| Rails/FindByOrAssignmentMemoization | Safe |
| Rails/FindEach | Safe |
| Rails/FreezeTime | Unsafe |
| Rails/HttpPositionalArguments | Safe |
| Rails/HttpStatus | Safe |
| Rails/HttpStatusNameConsistency | Safe |
| Rails/I18nLazyLookup | Safe |
| Rails/IgnoredColumnsAssignment | Unsafe |
| Rails/IgnoredSkipActionFilterOption | Safe |
| Rails/IndexBy | Safe |
| Rails/IndexWith | Unsafe |
| Rails/LinkToBlank | Safe |
| Rails/MailerName | Unsafe |
| Rails/MatchRoute | Safe |
| Rails/MigrationClassName | Safe |
| Rails/MultipleRoutePaths | Safe |
| Rails/NegateInclude | Safe |
| Rails/OrderArguments | Safe |
| Rails/Output | Unsafe |
| Rails/Pick | Safe |
| Rails/Pluck | Safe |
| Rails/PluckId | Safe |
| Rails/PluckInWhere | Safe |
| Rails/PluralizationGrammar | Safe |
| Rails/Presence | Safe |
| Rails/Present | Safe |
| Rails/RakeEnvironment | Safe |
| Rails/ReadWriteAttribute | Safe |
| Rails/RedirectBackOrTo | Safe |
| Rails/RedundantActiveRecordAllMethod | Safe |
| Rails/RedundantAllowNil | Safe |
| Rails/RedundantForeignKey | Safe |
| Rails/RedundantPresenceValidationOnBelongsTo | Unsafe |
| Rails/RedundantReceiverInWithOptions | Safe |
| Rails/RedundantTravelBack | Safe |
| Rails/ReflectionClassName | Safe |
| Rails/RefuteMethods | Safe |
| Rails/RelativeDateConstant | Unsafe |
| Rails/RenderPlainText | Safe |
| Rails/RequestReferer | Safe |
| Rails/ResponseParsedBody | Safe |
| Rails/RootJoinChain | Safe |
| Rails/RootPathnameMethods | Unsafe |
| Rails/RootPublicPath | Safe |
| Rails/SafeNavigation | Safe |
| Rails/SafeNavigationWithBlank | Unsafe |
| Rails/SaveBang | Unsafe |
| Rails/ScopeArgs | Safe |
| Rails/SelectMap | Safe |
| Rails/ShortI18n | Safe |
| Rails/SquishedSQLHeredocs | Unsafe |
| Rails/StripHeredoc | Safe |
| Rails/StrongParametersExpect | Unsafe |
| Rails/TimeZone | Unsafe |
| Rails/ToFormattedS | Safe |
| Rails/ToSWithArgument | Safe |
| Rails/TopLevelHashWithIndifferentAccess | Safe |
| Rails/UniqBeforePluck | Unsafe |
| Rails/Validation | Safe |
| Rails/WhereEquals | Unsafe |
| Rails/WhereExists | Unsafe |
| Rails/WhereMissing | Safe |
| Rails/WhereNot | Safe |
| Rails/WhereRange | Unsafe |

## RuboCop-RSpec (60 autocorrectable / 114 total)

| Cop | Safety |
|-----|--------|
| RSpec/AlignLeftLetBrace | Safe |
| RSpec/AlignRightLetBrace | Safe |
| RSpec/BeEmpty | Contextual |
| RSpec/BeEq | Safe |
| RSpec/BeEql | Safe |
| RSpec/BeNil | Safe |
| RSpec/ChangeByZero | Safe |
| RSpec/ClassCheck | Safe |
| RSpec/ContainExactly | Safe |
| RSpec/ContextMethod | Safe |
| RSpec/DescribedClass | Unsafe |
| RSpec/Dialect | Safe |
| RSpec/DuplicatedMetadata | Safe |
| RSpec/EmptyExampleGroup | Unsafe + Contextual |
| RSpec/EmptyHook | Contextual |
| RSpec/EmptyLineAfterExample | Safe |
| RSpec/EmptyLineAfterExampleGroup | Safe |
| RSpec/EmptyLineAfterFinalLet | Safe |
| RSpec/EmptyLineAfterHook | Safe |
| RSpec/EmptyLineAfterSubject | Safe |
| RSpec/EmptyMetadata | Contextual |
| RSpec/EmptyOutput | Safe |
| RSpec/Eq | Safe |
| RSpec/ExampleWording | Safe |
| RSpec/ExcessiveDocstringSpacing | Safe |
| RSpec/ExpectActual | Safe |
| RSpec/ExpectChange | Unsafe |
| RSpec/Focus | Contextual |
| RSpec/HookArgument | Safe |
| RSpec/HooksBeforeExamples | Contextual |
| RSpec/ImplicitExpect | Safe |
| RSpec/ImplicitSubject | Safe |
| RSpec/IncludeExamples | Unsafe |
| RSpec/InstanceSpy | Safe |
| RSpec/IsExpectedSpecify | Safe |
| RSpec/ItBehavesLike | Safe |
| RSpec/IteratedExpectation | Safe |
| RSpec/LeadingSubject | Safe |
| RSpec/LetBeforeExamples | Contextual |
| RSpec/MatchArray | Safe |
| RSpec/MetadataStyle | Safe |
| RSpec/MultipleSubjects | Safe |
| RSpec/NotToNot | Safe |
| RSpec/Output | Unsafe + Contextual |
| RSpec/PredicateMatcher | Unsafe |
| RSpec/ReceiveCounts | Safe |
| RSpec/ReceiveMessages | Unsafe |
| RSpec/ReceiveNever | Safe |
| RSpec/RedundantAround | Safe |
| RSpec/RedundantPredicateMatcher | Safe |
| RSpec/ReturnFromStub | Safe |
| RSpec/ScatteredLet | Contextual |
| RSpec/ScatteredSetup | Contextual |
| RSpec/SharedContext | Safe |
| RSpec/SharedExamples | Safe |
| RSpec/SingleArgumentMessageChain | Safe |
| RSpec/SortMetadata | Safe |
| RSpec/VariableDefinition | Safe |
| RSpec/VerifiedDoubleReference | Unsafe |
| RSpec/Yield | Safe |

## Non-Autocorrectable Departments

| Department | Total | Autocorrectable | Detection-Only |
|-----------|-------|----------------|----------------|
| Metrics | 10 | 0 | 10 (100%) |
| Naming | 19 | 6 | 13 (68%) |
| Security | 7 | 3 | 4 (57%) |
| Bundler | 7 | 2 | 5 (71%) |
| Gemspec | 10 | 4 | 6 (60%) |
| Lint | 152 | 85 | 67 (44%) |
| RSpec | 114 | 60 | 54 (47%) |
| Rails | 148 | 101 | 47 (32%) |
| Style | 287 | 258 | 29 (10%) |
| Layout | 100 | 99 | 1 (1%) |

Metrics cops (CyclomaticComplexity, MethodLength, etc.) are inherently non-autocorrectable — there's no mechanical fix for "method is too long."

---

## Extraction Scripts

The tables above were generated from the vendor submodules. Here are the scripts to reproduce or update them.

### Count autocorrectable cops per gem

```bash
for gem in rubocop rubocop-performance rubocop-rails rubocop-rspec; do
    total=$(grep -cE '^\w+/\w+:' vendor/$gem/config/default.yml 2>/dev/null || echo 0)
    autocorrect=$(grep -rl "extend AutoCorrector" vendor/$gem/lib/ 2>/dev/null \
        | grep -v '/base\.rb$' | grep -v 'internal_affairs' | grep -v '/cop\.rb$' | wc -l | tr -d ' ')
    echo "$gem: $total total cops in config, $autocorrect with AutoCorrector"
done
```

### Full cop list with safety classifications

```python
#!/usr/bin/env python3
"""Extract all autocorrectable cops with safety classifications from vendor submodules."""

import subprocess, re

def get_autocorrectable_cops(lib_path):
    """Get all cops that extend AutoCorrector from a gem."""
    result = subprocess.run(
        ['grep', '-rl', 'extend AutoCorrector', lib_path],
        capture_output=True, text=True
    )
    cops = []
    for line in result.stdout.strip().split('\n'):
        if not line or '/base.rb' in line or '/internal_affairs/' in line or line.endswith('/cop.rb'):
            continue
        pattern = rf'{re.escape(lib_path)}/(\w+)/(\w+)\.rb'
        m = re.search(pattern, line)
        if m:
            dept = ''.join(w.capitalize() for w in m.group(1).split('_'))
            cop = ''.join(w.capitalize() for w in m.group(2).split('_'))
            cops.append(f'{dept}/{cop}')
    return sorted(cops)

def get_safety_config(config_path):
    """Parse default.yml for SafeAutoCorrect and AutoCorrect settings."""
    unsafe = set()
    contextual = set()
    current_cop = None
    with open(config_path) as f:
        for line in f:
            m = re.match(r'^(\w+/\w+):', line)
            if m:
                current_cop = m.group(1)
            if current_cop and 'SafeAutoCorrect: false' in line:
                unsafe.add(current_cop)
            if current_cop and 'AutoCorrect: contextual' in line:
                contextual.add(current_cop)
    return unsafe, contextual

gems = [
    ('rubocop',             'vendor/rubocop/lib/rubocop/cop',             'vendor/rubocop/config/default.yml'),
    ('rubocop-performance', 'vendor/rubocop-performance/lib/rubocop/cop', 'vendor/rubocop-performance/config/default.yml'),
    ('rubocop-rails',       'vendor/rubocop-rails/lib/rubocop/cop',       'vendor/rubocop-rails/config/default.yml'),
    ('rubocop-rspec',       'vendor/rubocop-rspec/lib/rubocop/cop',       'vendor/rubocop-rspec/config/default.yml'),
]

for gem_name, lib_path, config_path in gems:
    cops = get_autocorrectable_cops(lib_path)
    unsafe, contextual = get_safety_config(config_path)
    print(f"\n=== {gem_name} ({len(cops)} autocorrectable) ===")
    for cop in cops:
        safety = "safe"
        if cop in unsafe and cop in contextual:
            safety = "unsafe+contextual"
        elif cop in unsafe:
            safety = "unsafe"
        elif cop in contextual:
            safety = "contextual"
        print(f"  {cop} [{safety}]")
```

### List cops WITHOUT autocorrect (detection-only)

```bash
# Core rubocop: find all cop files that do NOT contain "extend AutoCorrector"
grep -rL "extend AutoCorrector" vendor/rubocop/lib/rubocop/cop/*/*.rb \
    | grep -v '/base\.rb$' \
    | sed 's|vendor/rubocop/lib/rubocop/cop/||; s|\.rb$||' \
    | sort
```

### Count by department

```bash
grep -rl "extend AutoCorrector" vendor/rubocop/lib/rubocop/cop/ \
    | grep -v '/base\.rb$' | grep -v 'internal_affairs' | grep -v '/cop\.rb$' \
    | sed 's|vendor/rubocop/lib/rubocop/cop/||; s|\.rb$||' \
    | awk -F/ '{print $1}' | sort | uniq -c | sort -rn
```
