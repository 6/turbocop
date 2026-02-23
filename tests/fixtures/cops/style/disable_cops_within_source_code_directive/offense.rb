def foo # rubocop:disable Metrics/CyclomaticComplexity
        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DisableCopsWithinSourceCodeDirective: RuboCop disable/enable directives are not permitted.
end

def bar # rubocop:enable Metrics/CyclomaticComplexity
        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DisableCopsWithinSourceCodeDirective: RuboCop disable/enable directives are not permitted.
end

def baz # rubocop:enable all
        ^^^^^^^^^^^^^^^^^^^^ Style/DisableCopsWithinSourceCodeDirective: RuboCop disable/enable directives are not permitted.
end
