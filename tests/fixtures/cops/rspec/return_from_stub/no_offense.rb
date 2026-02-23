it do
  allow(Foo).to receive(:bar) { baz }
end
it do
  allow(Foo).to receive(:bar).and_return(42)
end
it do
  allow(Foo).to receive(:bar)
end
it do
  allow(Foo).to receive(:bar) { [42, baz] }
end
it do
  bar = 42
  allow(Foo).to receive(:bar) { bar }
end
# Constants are not static values — they can change at runtime
it do
  allow(Foo).to receive(:bar) { SomeConstant }
end
it do
  allow(Foo).to receive(:bar) { Module::CONSTANT }
end
