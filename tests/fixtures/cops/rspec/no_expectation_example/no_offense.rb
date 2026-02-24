RSpec.describe Foo do
  it { expect(baz).to be_truthy }

  it { should be_valid }

  it { should_not be_empty }

  it 'has an expectation' do
    expect(subject.name).to eq('foo')
  end

  it 'uses is_expected' do
    is_expected.to be_present
  end

  it 'uses are_expected' do
    are_expected.to all(be_positive)
  end

  it 'uses should_receive' do
    should_receive(:foo)
  end

  it 'uses should_not_receive' do
    should_not_receive(:bar)
  end

  it 'not implemented'

  # assert_* methods are recognized as expectations (name.starts_with("assert"))
  it 'runs assert_difference' do
    assert_difference 'User.count', 1 do
      create_user
    end
  end

  it 'uses assert_equal' do
    assert_equal(expected, actual)
  end
end
