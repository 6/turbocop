expect(foo).to receive(:bar).never
                             ^^^^^ RSpec/ReceiveNever: Use `not_to receive` instead of `never`.

expect(foo).to receive(:bar).with(1).never
                                     ^^^^^ RSpec/ReceiveNever: Use `not_to receive` instead of `never`.

expect(foo).to receive(:baz).never
                             ^^^^^ RSpec/ReceiveNever: Use `not_to receive` instead of `never`.

expect_any_instance_of(klass).not_to receive(:not_expected).never
                                                            ^^^^^ RSpec/ReceiveNever: Use `not_to receive` instead of `never`.

expect(@double).not_to receive(:not_expected).never
                                              ^^^^^ RSpec/ReceiveNever: Use `not_to receive` instead of `never`.

expect(@double).to receive(:do_something).never.and_return(1)
                                          ^^^^^ RSpec/ReceiveNever: Use `not_to receive` instead of `never`.

wrapped.not_to receive(:foo).never
                             ^^^^^ RSpec/ReceiveNever: Use `not_to receive` instead of `never`.

wrapped.to_not receive(:foo).never
                             ^^^^^ RSpec/ReceiveNever: Use `not_to receive` instead of `never`.
