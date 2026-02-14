it { is_expected.to contain_exactly(*array1, *array2) }
                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ContainExactly: Prefer `match_array` when matching array values.
it { is_expected.to contain_exactly(*[1, 2, 3]) }
                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ContainExactly: Prefer `match_array` when matching array values.
it { is_expected.to contain_exactly(*a.merge(b)) }
                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ContainExactly: Prefer `match_array` when matching array values.
