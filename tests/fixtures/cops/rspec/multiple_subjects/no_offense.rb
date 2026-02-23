describe Foo do
  it_behaves_like 'user' do
    subject { described_class.new(user, described_class) }

    it { expect(subject).not_to be_accessible }
  end

  it_behaves_like 'admin' do
    subject { described_class.new(user, described_class) }

    it { expect(subject).to be_accessible }
  end
end
