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
