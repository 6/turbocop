it { expect(false).to_not be_true }
                   ^^^^^^ RSpec/NotToNot: Prefer `not_to` over `to_not`.
it { expect(nil).to_not be_nil }
                 ^^^^^^ RSpec/NotToNot: Prefer `not_to` over `to_not`.
it { expect(0).to_not eq(1) }
               ^^^^^^ RSpec/NotToNot: Prefer `not_to` over `to_not`.
