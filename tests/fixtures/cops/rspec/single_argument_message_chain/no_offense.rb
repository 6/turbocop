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
  allow(controller).to receive_message_chain "forum.moderator?" => false
end

before do
  controller.stub_chain "admin?" => true
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

it "normal users cannot access moderation" do
  allow(controller).to receive_message_chain "forum.moderator?" => false

  get :index, forum_id: 1
  expect(flash[:alert]).to eq("You are not allowed to do that.")
end

it "moderators can access moderation" do
  allow(controller).to receive_message_chain "forum.moderator?" => true
  get :index, forum_id: 1
  expect(flash[:alert]).to be_nil
end
