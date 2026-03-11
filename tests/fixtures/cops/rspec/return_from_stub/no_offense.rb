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
# receive_message_chain with block is not flagged by RuboCop
it do
  allow(order).to receive_message_chain(:payments, :valid, :empty?) { false }
end
it do
  allow(obj).to receive_message_chain(:foo, :bar) { 42 }
end
# .freeze on a dynamic value is still dynamic
it do
  allow(Foo).to receive(:bar) { some_method.freeze }
end
# Block with parameter is dynamic
it do
  allow(Foo).to receive(:bar) { |arg| arg }
end
