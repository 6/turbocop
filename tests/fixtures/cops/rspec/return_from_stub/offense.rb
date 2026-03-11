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
# Constants are static values (recursive_literal_or_const?)
it do
  allow(Foo).to receive(:bar) { SomeConstant }
                              ^ RSpec/ReturnFromStub: Use `and_return` for static values.
end
it do
  allow(Foo).to receive(:bar) { Module::CONSTANT }
                              ^ RSpec/ReturnFromStub: Use `and_return` for static values.
end
it do
  allow(Foo).to receive(:bar) { {Life::MEANING => 42} }
                              ^ RSpec/ReturnFromStub: Use `and_return` for static values.
end
# Ranges are static values
it do
  allow(Foo).to receive(:bar) { 1..10 }
                              ^ RSpec/ReturnFromStub: Use `and_return` for static values.
end
# Regexps are static values
it do
  allow(Foo).to receive(:bar) { /pattern/ }
                              ^ RSpec/ReturnFromStub: Use `and_return` for static values.
end
# Rational and imaginary literals are static values
it do
  allow(Foo).to receive(:bar) { 1r }
                              ^ RSpec/ReturnFromStub: Use `and_return` for static values.
end
it do
  allow(Foo).to receive(:bar) { 1i }
                              ^ RSpec/ReturnFromStub: Use `and_return` for static values.
end
# .freeze on a literal is still static
it do
  allow(Foo).to receive(:bar) { "foo".freeze }
                              ^ RSpec/ReturnFromStub: Use `and_return` for static values.
end
