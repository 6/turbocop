# Receiver is a send node (regular method call with `.`) — RuboCop flags these
return unless foo.bar&.empty?
              ^^^^^^^^^^^^^^^ Lint/SafeNavigationWithEmpty: Avoid calling `empty?` with the safe navigation operator in conditionals.
bar if collection.find_all&.empty?
       ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/SafeNavigationWithEmpty: Avoid calling `empty?` with the safe navigation operator in conditionals.
do_something if items.select&.empty?
                ^^^^^^^^^^^^^^^^^^^^^^^^ Lint/SafeNavigationWithEmpty: Avoid calling `empty?` with the safe navigation operator in conditionals.
