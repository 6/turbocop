return unless foo&.empty?
              ^^^^^^^^^^^ Lint/SafeNavigationWithEmpty: Avoid calling `empty?` with the safe navigation operator in conditionals.
bar if baz&.empty?
       ^^^^^^^^^^^ Lint/SafeNavigationWithEmpty: Avoid calling `empty?` with the safe navigation operator in conditionals.
x&.empty?
^^^^^^^^^ Lint/SafeNavigationWithEmpty: Avoid calling `empty?` with the safe navigation operator in conditionals.
