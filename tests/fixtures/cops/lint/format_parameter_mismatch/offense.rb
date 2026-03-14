format("%s %s", 1)
^^^^^^ Lint/FormatParameterMismatch: Number of arguments (1) to `format` doesn't match the number of fields (2).
sprintf("%s %s", 1, 2, 3)
^^^^^^^ Lint/FormatParameterMismatch: Number of arguments (3) to `sprintf` doesn't match the number of fields (2).
"%s %s" % [1, 2, 3]
        ^ Lint/FormatParameterMismatch: Number of arguments (3) to `String#%` doesn't match the number of fields (2).
format("something", 1)
^^^^^^ Lint/FormatParameterMismatch: Number of arguments (1) to `format` doesn't match the number of fields (0).
Kernel.format("%s %s", 1)
       ^^^^^^ Lint/FormatParameterMismatch: Number of arguments (1) to `format` doesn't match the number of fields (2).
Kernel.sprintf("%s %s", 1)
       ^^^^^^^ Lint/FormatParameterMismatch: Number of arguments (1) to `sprintf` doesn't match the number of fields (2).
format("<< %s bleh", 1, 2)
^^^^^^ Lint/FormatParameterMismatch: Number of arguments (2) to `format` doesn't match the number of fields (1).
format('%1$s %2$s', 'foo', 'bar', 'baz')
^^^^^^ Lint/FormatParameterMismatch: Number of arguments (3) to `format` doesn't match the number of fields (2).
format('%s %2$s', 'foo', 'bar')
^^^^^^ Lint/FormatParameterMismatch: Format string is invalid because formatting sequence types (numbered, named or unnumbered) are mixed.
"%*2$s" % [5, 5, 5]
        ^ Lint/FormatParameterMismatch: Number of arguments (3) to `String#%` doesn't match the number of fields (2).
"%*.*2$s" % [5, 5, 5]
          ^ Lint/FormatParameterMismatch: Format string is invalid because formatting sequence types (numbered, named or unnumbered) are mixed.
"%*2$.*2$s" % [5, 5, 5]
            ^ Lint/FormatParameterMismatch: Number of arguments (3) to `String#%` doesn't match the number of fields (2).
"%{foo}" % []
         ^ Lint/FormatParameterMismatch: Number of arguments (0) to `String#%` doesn't match the number of fields (1).
format('text/plain', &:inspect)
^^^^^^ Lint/FormatParameterMismatch: Number of arguments (1) to `format` doesn't match the number of fields (0).
