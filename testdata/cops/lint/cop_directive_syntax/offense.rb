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
