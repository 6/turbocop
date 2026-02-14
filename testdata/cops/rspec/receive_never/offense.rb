expect(foo).to receive(:bar).never
                             ^^^^^ RSpec/ReceiveNever: Use `not_to receive` instead of `never`.

expect(foo).to receive(:bar).with(1).never
                                     ^^^^^ RSpec/ReceiveNever: Use `not_to receive` instead of `never`.

expect(foo).to receive(:baz).never
                             ^^^^^ RSpec/ReceiveNever: Use `not_to receive` instead of `never`.
