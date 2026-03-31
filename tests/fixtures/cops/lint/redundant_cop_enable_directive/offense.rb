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

alpha
# rubocop: enable Metrics/CyclomaticComplexity
                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/RedundantCopEnableDirective: Unnecessary enabling of Metrics/CyclomaticComplexity.

beta
# rubocop:enable MethodLength
                 ^^^^^^^^^^^^ Lint/RedundantCopEnableDirective: Unnecessary enabling of MethodLength.

def help # rubocop:disable MethodLength
  <<~TEXT
    body
  TEXT
end
# rubocop:enable MethodLength
                 ^^^^^^^^^^^^ Lint/RedundantCopEnableDirective: Unnecessary enabling of MethodLength.
