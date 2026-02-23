before do
  allow(foo).to receive_message_chain(:one, :two) { :three }
end

before do
  allow(foo).to receive_message_chain("one.two") { :three }
end

before do
  foo.stub_chain(:one, :two) { :three }
end

before do
  allow(foo).to receive(:one) { :two }
end
