RSpec.describe Foo do
  it 'is short enough' do
    expect(1).to eq(1)
    expect(2).to eq(2)
  end

  it 'has exactly five lines' do
    a = 1
    b = 2
    c = 3
    d = 4
    expect(a + b + c + d).to eq(10)
  end

  it { expect(true).to be(true) }

  specify do
    expect(subject).to be_valid
  end
end
