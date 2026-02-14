describe FooClass do
  it 'works' do
    expect(true).to eq(true)
  end
end

RSpec.describe FooClass do
  context 'when valid' do
    it 'succeeds' do
      expect(subject).to be_valid
    end
  end
end
