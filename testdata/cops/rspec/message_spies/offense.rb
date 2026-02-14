expect(foo).to receive(:bar)
               ^^^^^^^ RSpec/MessageSpies: Prefer `have_received` for setting message expectations. Setup the object as a spy using `allow` or `instance_spy`.
expect(foo).not_to receive(:bar)
                   ^^^^^^^ RSpec/MessageSpies: Prefer `have_received` for setting message expectations. Setup the object as a spy using `allow` or `instance_spy`.
expect(foo).to_not receive(:baz)
                   ^^^^^^^ RSpec/MessageSpies: Prefer `have_received` for setting message expectations. Setup the object as a spy using `allow` or `instance_spy`.
