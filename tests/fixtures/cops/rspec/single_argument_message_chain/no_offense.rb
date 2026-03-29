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

before do
  allow(controller).to receive_message_chain(
    "forum.moderator?" => false,
    "forum.admin?" => true
  )
end

before do
  controller.stub_chain(
    "admin?" => true,
    "staff?" => false
  )
end
