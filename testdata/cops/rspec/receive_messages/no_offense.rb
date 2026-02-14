before do
  allow(Foo).to receive(:foo).and_return(baz)
  allow(Bar).to receive(:bar).and_return(bar)
  allow(Baz).to receive(:baz).and_return(foo)
end
before do
  allow(Service).to receive(:foo) { baz }
  allow(Service).to receive(:bar) { bar }
end
