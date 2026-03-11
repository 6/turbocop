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

# its() with different string attributes but same block body are NOT duplicates
# The first string arg to its() is an attribute accessor, not a description
describe docker_container(name: 'an-echo-server') do
  its('Server.Version') { should cmp >= '1.12' }
  its('Client.Version') { should cmp >= '1.12' }
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

# Tag metadata makes examples non-duplicate even with same body
describe 'doing x' do
  it "does x" do
    expect(foo).to be(bar)
  end

  it "does y", :focus do
    expect(foo).to be(bar)
  end
end

# Repeated examples in different scopes are NOT duplicates
describe 'doing x' do
  it "does x" do
    expect(foo).to be(bar)
  end

  context 'when the scope changes' do
    it 'does not flag anything' do
      expect(foo).to be(bar)
    end
  end
end

# Nested contexts with same implementation in each — NOT duplicates
describe 'doing x' do
  context 'context A' do
    it "does x" do
      expect(foo).to be(bar)
    end
  end

  context 'context B' do
    it "does x" do
      expect(foo).to be(bar)
    end
  end
end

# its() with different block expectations
describe 'doing x' do
  its(:x) { is_expected.to be_present }
  its(:x) { is_expected.to be_blank }
end

# Block-less example calls with same metadata are NOT duplicates
# RuboCop requires a block to consider something an example
describe 'pending examples' do
  it "is pending"
  it "is also pending"
end

# Examples with a receiver are NOT detected (RuboCop requires nil receiver)
describe 'receiver examples' do
  object.it { expect(foo).to be(bar) }
  object.it { expect(foo).to be(bar) }
end

# Argless example and named example with same body are NOT duplicates
# RuboCop distinguishes nil metadata (no args) from [] metadata (has doc string)
describe 'argless vs named' do
  it { expect(foo).to be(bar) }
  it "named" do
    expect(foo).to be(bar)
  end
end

# Block-pass examples (it(&proc)) are NOT examples per RuboCop
# RuboCop's example? matcher requires (block ...), not (send ... block_pass)
describe 'block pass examples' do
  it(&method(:validate_name))
  it(&method(:validate_email))
end

# Safe navigation (&.) vs regular dot should produce different fingerprints
# RuboCop uses csend for &. and send for . — different AST node types
describe 'safe navigation difference' do
  it "regular" do
    expect(user.name).to be_present
  end
  it "safe nav" do
    expect(user&.name).to be_present
  end
end
