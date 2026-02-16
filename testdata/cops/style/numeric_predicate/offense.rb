foo == 0
^^^^^^^^ Style/NumericPredicate: Use `foo.zero?` instead of `foo == 0`.

bar.baz > 0
^^^^^^^^^^^ Style/NumericPredicate: Use `bar.baz.positive?` instead of `bar.baz > 0`.

0 > foo
^^^^^^^ Style/NumericPredicate: Use `foo.negative?` instead of `0 > foo`.
