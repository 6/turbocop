foo
# rubocop:enable Layout/LineLength
                 ^^^^^^^^^^^^^^^^^ Lint/RedundantCopEnableDirective: Unnecessary enabling of Layout/LineLength.
bar
# rubocop:enable Metrics/ModuleLength, Metrics/AbcSize
                                       ^^^^^^^^^^^^^^^ Lint/RedundantCopEnableDirective: Unnecessary enabling of Metrics/AbcSize.
                 ^^^^^^^^^^^^^^^^^^^^ Lint/RedundantCopEnableDirective: Unnecessary enabling of Metrics/ModuleLength.
baz
# rubocop:enable all
                 ^^^ Lint/RedundantCopEnableDirective: Unnecessary enabling of all cops.
