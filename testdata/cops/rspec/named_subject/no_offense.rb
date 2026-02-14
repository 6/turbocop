RSpec.describe User do
  subject(:user) { described_class.new }

  it "is a User" do
    expect(user).to be_a(User)
  end

  it "is valid" do
    expect(user.valid?).to be(true)
  end

  subject { described_class.new }
end
