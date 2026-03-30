def foo # rubocop:disable Metrics/CyclomaticComplexity
        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DisableCopsWithinSourceCodeDirective: RuboCop disable/enable directives are not permitted.
end

def bar # rubocop:enable Metrics/CyclomaticComplexity
        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DisableCopsWithinSourceCodeDirective: RuboCop disable/enable directives are not permitted.
end

def baz # rubocop:enable all
        ^^^^^^^^^^^^^^^^^^^^ Style/DisableCopsWithinSourceCodeDirective: RuboCop disable/enable directives are not permitted.
end

# rubocop:disable Metrics/MethodLength
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DisableCopsWithinSourceCodeDirective: RuboCop disable/enable directives are not permitted.
def long_method
  puts "a"
end
# rubocop:enable Metrics/MethodLength
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DisableCopsWithinSourceCodeDirective: RuboCop disable/enable directives are not permitted.

# rubocop:todo Metrics/AbcSize
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DisableCopsWithinSourceCodeDirective: RuboCop disable/enable directives are not permitted.
def complex_method
  x = 1
end

"name" => File.basename(file_input.filename, File.extname(file_input.filename)), # AWS requires a filename and it cannot include dots from the extension # rubocop:disable Layout/LineLength
                                                                                 ^ Style/DisableCopsWithinSourceCodeDirective: RuboCop disable/enable directives are not permitted.

self #: as untyped # rubocop:disable Style/RedundantSelf
     ^ Style/DisableCopsWithinSourceCodeDirective: RuboCop disable/enable directives are not permitted.

@@schemes = @@schemes #: Hash[String, untyped] # rubocop:disable Style/ClassVars
                      ^ Style/DisableCopsWithinSourceCodeDirective: RuboCop disable/enable directives are not permitted.

# comment on comment # rubocop:disable Layout/CommentIndentation
^ Style/DisableCopsWithinSourceCodeDirective: RuboCop disable/enable directives are not permitted.

range = self #: as untyped # rubocop:disable Style/RedundantSelf
             ^ Style/DisableCopsWithinSourceCodeDirective: RuboCop disable/enable directives are not permitted.

range = self #: as untyped # rubocop:disable Style/RedundantSelf
             ^ Style/DisableCopsWithinSourceCodeDirective: RuboCop disable/enable directives are not permitted.

range = self #: as untyped # rubocop:disable Style/RedundantSelf
             ^ Style/DisableCopsWithinSourceCodeDirective: RuboCop disable/enable directives are not permitted.

range = self #: as untyped # rubocop:disable Style/RedundantSelf
             ^ Style/DisableCopsWithinSourceCodeDirective: RuboCop disable/enable directives are not permitted.
