expect(foo).to receive(:bar)
               ^^^^^^^ RSpec/MessageSpies: Prefer `have_received` for setting message expectations. Setup `foo` as a spy using `allow` or `instance_spy`.
expect(foo).not_to receive(:bar)
                   ^^^^^^^ RSpec/MessageSpies: Prefer `have_received` for setting message expectations. Setup `foo` as a spy using `allow` or `instance_spy`.
expect(foo).to_not receive(:baz)
                   ^^^^^^^ RSpec/MessageSpies: Prefer `have_received` for setting message expectations. Setup `foo` as a spy using `allow` or `instance_spy`.
expect(foo).to receive(:bar).with(:baz)
               ^^^^^^^ RSpec/MessageSpies: Prefer `have_received` for setting message expectations. Setup `foo` as a spy using `allow` or `instance_spy`.
expect(foo).to receive(:bar).at_most(42).times
               ^^^^^^^ RSpec/MessageSpies: Prefer `have_received` for setting message expectations. Setup `foo` as a spy using `allow` or `instance_spy`.
expect(foo).to receive(:bar).and receive(:baz)
               ^^^^^^^ RSpec/MessageSpies: Prefer `have_received` for setting message expectations. Setup `foo` as a spy using `allow` or `instance_spy`.
                                 ^^^^^^^ RSpec/MessageSpies: Prefer `have_received` for setting message expectations. Setup `foo` as a spy using `allow` or `instance_spy`.

expect(allow(test_double).to receive(:foo)).to have_string_representation("x")
                             ^^^^^^^ RSpec/MessageSpies: Prefer `have_received` for setting message expectations. Setup `allow(test_double).to receive(:foo)` as a spy using `allow` or `instance_spy`.

expect(allow("partial double".dup).to receive(:foo)).to have_string_representation("x")
                                      ^^^^^^^ RSpec/MessageSpies: Prefer `have_received` for setting message expectations. Setup `allow("partial double".dup).to receive(:foo)` as a spy using `allow` or `instance_spy`.

expect(allow(test_double).to receive(:foo).with(1, a_kind_of(String), any_args)).to have_string_representation("x")
                             ^^^^^^^ RSpec/MessageSpies: Prefer `have_received` for setting message expectations. Setup `allow(test_double).to receive(:foo).with(1, a_kind_of(String), any_args)` as a spy using `allow` or `instance_spy`.
