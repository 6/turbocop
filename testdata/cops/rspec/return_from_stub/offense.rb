it do
  allow(Foo).to receive(:bar) { 42 }
                              ^ RSpec/ReturnFromStub: Use `and_return` for static values.
end
it do
  allow(Foo).to receive(:baz) {}
                              ^ RSpec/ReturnFromStub: Use `and_return` for static values.
end
it do
  allow(Foo).to receive(:qux) { [1, 2] }
                              ^ RSpec/ReturnFromStub: Use `and_return` for static values.
end
