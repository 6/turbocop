it { expect(foo).to be == true }
                    ^^ RSpec/Eq: Use `eq` instead of `be ==` to compare objects.

it { expect(bar).not_to be == 1 }
                        ^^ RSpec/Eq: Use `eq` instead of `be ==` to compare objects.

it { expect(baz).to be == "hello" }
                    ^^ RSpec/Eq: Use `eq` instead of `be ==` to compare objects.
