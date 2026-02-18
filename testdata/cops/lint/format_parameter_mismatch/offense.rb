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
