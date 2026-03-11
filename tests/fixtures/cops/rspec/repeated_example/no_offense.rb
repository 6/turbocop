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

# Safe navigation (&.) vs regular (.) calls are NOT duplicates in RuboCop
# RuboCop uses (send ...) vs (csend ...) — different node types
describe 'safe navigation' do
  it { expect(user.name).to eq('John') }
  it { expect(user&.name).to eq('John') }
end

# Examples with different operator assignments are NOT duplicates
# (x += 1) vs (y += 1) differ by variable name in RuboCop AST
describe 'operator assignments' do
  it do
    count += 1
    expect(count).to eq(2)
  end
  it do
    total += 1
    expect(total).to eq(2)
  end
end

# Operator assignments with different target vars but same result reference
# The operator write node name differs (count vs total) even if rest is same
describe 'operator assign diff target' do
  it do
    count += 1
    expect(result).to eq(2)
  end
  it do
    total += 1
    expect(result).to eq(2)
  end
end

# Examples with different inclusive/exclusive ranges are NOT duplicates
# 1..10 is (irange ...) vs 1...10 is (erange ...) in RuboCop AST
describe 'range types' do
  it { expect(1..10).to include(5) }
  it { expect(1...10).to include(5) }
end

# Examples with different multi-assignment targets are NOT duplicates
describe 'multi-assignment' do
  it do
    first, _ = values
    expect(first).to eq(1)
  end
  it do
    _, second = values
    expect(second).to eq(2)
  end
end

# Examples with nested blocks having different unused params are NOT duplicates
# RuboCop AST includes (arg :a) vs (arg :b) in structural comparison
describe 'nested block params' do
  it { expect { |a| run }.to yield_control }
  it { expect { |b| run }.to yield_control }
end
