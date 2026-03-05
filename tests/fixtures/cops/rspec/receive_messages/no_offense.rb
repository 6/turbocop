before do
  allow(Foo).to receive(:foo).and_return(baz)
  allow(Bar).to receive(:bar).and_return(bar)
  allow(Baz).to receive(:baz).and_return(foo)
end
before do
  allow(Service).to receive(:foo) { baz }
  allow(Service).to receive(:bar) { bar }
end
# String args to receive() should not trigger receive_messages
before(:each) do
  allow(self).to receive('action_name').and_return(action_name)
  allow(self).to receive('current_page?').and_return(false)
end
# Chaining .once/.twice after and_return should not trigger
before do
  allow(Service).to receive(:foo).and_return(1).once
  allow(Service).to receive(:bar).and_return(2).twice
  allow(Service).to receive(:baz).and_return(3).exactly(3).times
end
# .ordered after and_return should not trigger
before do
  allow(Service).to receive(:foo).and_return(1).ordered
  allow(Service).to receive(:bar).and_return(2).ordered
end
# Multiple and_return args should not trigger
before do
  allow(Service).to receive(:foo).and_return(1, 2)
  allow(Service).to receive(:bar).and_return(3, 4)
end
# Splat in and_return should not trigger
before do
  allow(Service).to receive(:foo).and_return(*array)
  allow(Service).to receive(:bar).and_return(*array)
end
# and_call_original should not trigger
before do
  allow(Service).to receive(:foo).and_call_original
  allow(Service).to receive(:bar).and_return(qux)
  allow(Service).to receive(:baz).and_call_original
end
# .with should not trigger
before do
  allow(Service).to receive(:foo).with(1).and_return(baz)
  allow(Service).to receive(:bar).with(2).and_return(bar)
end
# Same message (duplicate receive arg) should not trigger
before do
  allow(Foo).to receive(:foo).and_return(bar)
  allow(Foo).to receive(:foo).and_return(baz)
  allow(Foo).to receive(:bar).and_return(qux)
end
