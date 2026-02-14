RSpec.describe Foo do
  it { expect(baz).to be_truthy }

  it { should be_valid }

  it 'has an expectation' do
    expect(subject.name).to eq('foo')
  end

  it 'uses is_expected' do
    is_expected.to be_present
  end

  it 'not implemented'
end
