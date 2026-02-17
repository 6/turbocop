describe 'doing x' do
  it "does x" do
    expect(foo).to have_attribute(foo: 1)
  end

  it "does y" do
    expect(foo).to have_attribute(bar: 2)
  end
end

describe 'doing z' do
  its(:x) { is_expected.to be_present }
  its(:y) { is_expected.to be_present }
end

# Repeated examples inside shared_examples are NOT checked by RuboCop
# (shared_examples is a SharedGroup, not an ExampleGroup)
shared_examples 'common' do
  it 'does thing one' do
    expect_no_offenses('a = 1')
  end

  it 'does thing two' do
    expect_no_offenses('a = 1')
  end
end

# Heredoc examples with different content are NOT duplicates
# even though the StatementsNode source looks the same
describe 'heredoc examples' do
  it 'test1' do
    expect_no_offenses(<<~RUBY)
      spec.metadata['key-0'] = 'value-0'
    RUBY
  end

  it 'test2' do
    expect_no_offenses(<<~RUBY)
      spec.authors = %w[author-1 author-2]
    RUBY
  end

  it 'test3' do
    expect_no_offenses(<<~RUBY)
      completely_different_method_call
    RUBY
  end
end
