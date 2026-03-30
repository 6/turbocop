# rubocop:disable Layout/LineLength Style/Encoding
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/CopDirectiveSyntax: Malformed directive comment detected. Cop names must be separated by commas. Comment in the directive must start with `--`.
# rubocop:disable
^^^^^^^^^^^^^^^^^ Lint/CopDirectiveSyntax: Malformed directive comment detected. The cop name is missing.
# rubocop:disabled Layout/LineLength
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/CopDirectiveSyntax: Malformed directive comment detected. The mode name must be one of `enable`, `disable`, `todo`, `push`, or `pop`.
# rubocop:
^^^^^^^^^^ Lint/CopDirectiveSyntax: Malformed directive comment detected. The mode name is missing.
# rubocop:disable Layout/LineLength == This is a bad comment.
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/CopDirectiveSyntax: Malformed directive comment detected. Cop names must be separated by commas. Comment in the directive must start with `--`.
# rubocop:disable Layout:LineLength
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/CopDirectiveSyntax: Malformed directive comment detected. Cop names must be separated by commas. Comment in the directive must start with `--`.
# rubocop:enable Layout:LineLength
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/CopDirectiveSyntax: Malformed directive comment detected. Cop names must be separated by commas. Comment in the directive must start with `--`.
# rubocop:disable Layout:LineLength, Style:Encoding
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/CopDirectiveSyntax: Malformed directive comment detected. Cop names must be separated by commas. Comment in the directive must start with `--`.
# rubocop:disable Rails::SkipsModelValidations
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/CopDirectiveSyntax: Malformed directive comment detected. Cop names must be separated by commas. Comment in the directive must start with `--`.
# rubocop:enable Rails/SkipsModelValidations:
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/CopDirectiveSyntax: Malformed directive comment detected. Cop names must be separated by commas. Comment in the directive must start with `--`.
# rubocop:disable Metrics/BlockLength:
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/CopDirectiveSyntax: Malformed directive comment detected. Cop names must be separated by commas. Comment in the directive must start with `--`.
# rubocop:disable Metrics/BlockLength(RuboCop)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/CopDirectiveSyntax: Malformed directive comment detected. Cop names must be separated by commas. Comment in the directive must start with `--`.
# rubocop:enable Rails/FindEach.
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/CopDirectiveSyntax: Malformed directive comment detected. Cop names must be separated by commas. Comment in the directive must start with `--`.
# rubocop:disable Naming/PredicatePrefix?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/CopDirectiveSyntax: Malformed directive comment detected. Cop names must be separated by commas. Comment in the directive must start with `--`.
# rubocop:disable /BlockLength, Metrics/
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/CopDirectiveSyntax: Malformed directive comment detected. Cop names must be separated by commas. Comment in the directive must start with `--`.
a = 1 # rubocop:disable Discourse/NoChdir because this is not part of the app
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/CopDirectiveSyntax: Malformed directive comment detected. Cop names must be separated by commas. Comment in the directive must start with `--`.
Dir.chdir("#{__dir__}/..") # rubocop:disable Discourse/NoChdir because this is not part of the app
                           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/CopDirectiveSyntax: Malformed directive comment detected. Cop names must be separated by commas. Comment in the directive must start with `--`.
def method(klass, cons = nil, &block) # rubocop:disable Metrics/PerceivedComplexity:
                                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/CopDirectiveSyntax: Malformed directive comment detected. Cop names must be separated by commas. Comment in the directive must start with `--`.
# rubocop:disable Style/NestedModifier, Style/IfUnlessModifierOfIfUnless:
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/CopDirectiveSyntax: Malformed directive comment detected. Cop names must be separated by commas. Comment in the directive must start with `--`.

# rubocop:disable Layout/LineLength,
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/CopDirectiveSyntax: Malformed directive comment detected. Cop names must be separated by commas. Comment in the directive must start with `--`.
