(variable & flags).positive?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/BitwisePredicate: Replace with `anybits?` for comparison with bit flags.

(variable & flags) > 0
^^^^^^^^^^^^^^^^^^^^^^ Style/BitwisePredicate: Replace with `anybits?` for comparison with bit flags.

(variable & flags) == 0
^^^^^^^^^^^^^^^^^^^^^^^ Style/BitwisePredicate: Replace with `nobits?` for comparison with bit flags.
