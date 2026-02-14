expect(foo).to receive(:bar)
^^^^^^ RSpec/MessageExpectation: Prefer `allow` for setting message expectations.
expect(foo).to receive(:baz).with(1)
^^^^^^ RSpec/MessageExpectation: Prefer `allow` for setting message expectations.
expect(obj).not_to receive(:qux)
^^^^^^ RSpec/MessageExpectation: Prefer `allow` for setting message expectations.
