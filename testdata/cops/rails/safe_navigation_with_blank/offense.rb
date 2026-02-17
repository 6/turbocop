do_something if foo&.blank?
                ^^^^^^^^^^^ Rails/SafeNavigationWithBlank: Avoid calling `blank?` with the safe navigation operator in conditionals.
bar unless baz&.blank?
           ^^^^^^^^^^^ Rails/SafeNavigationWithBlank: Avoid calling `blank?` with the safe navigation operator in conditionals.
x if obj&.blank?
     ^^^^^^^^^^^ Rails/SafeNavigationWithBlank: Avoid calling `blank?` with the safe navigation operator in conditionals.
