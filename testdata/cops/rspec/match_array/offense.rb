it { is_expected.to match_array([content1, content2]) }
                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/MatchArray: Prefer `contain_exactly` when matching an array literal.
it { is_expected.to match_array([*content1, content2]) }
                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/MatchArray: Prefer `contain_exactly` when matching an array literal.
it { is_expected.to match_array([1, 2, 3]) }
                    ^^^^^^^^^^^^^^^^^^^^^^ RSpec/MatchArray: Prefer `contain_exactly` when matching an array literal.
