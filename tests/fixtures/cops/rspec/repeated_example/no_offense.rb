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

# Examples with different regex flags are NOT duplicates
# RuboCop distinguishes /foo/i from /foo/m in AST comparison
describe 'regex flags' do
  it { expect(str).to match(/pattern/i) }
  it { expect(str).to match(/pattern/m) }
end

# Regex with flags vs no flags are NOT duplicates
describe 'regex no flags' do
  it { expect(str).to match(/pattern/) }
  it { expect(str).to match(/pattern/i) }
end

# Interpolated regex with different flags are NOT duplicates
describe 'interpolated regex flags' do
  it { expect(str).to match(/#{prefix}value/i) }
  it { expect(str).to match(/#{prefix}value/m) }
end

# Match-last-line (/regex/ in conditional) with different flags
describe 'match last line flags' do
  it { if /pattern/i; expect(true).to eq("x"); end }
  it { if /pattern/m; expect(true).to eq("x"); end }
end

# Examples with different back references are NOT duplicates
# $& vs $` are different in RuboCop AST (back_ref :$& vs back_ref :$`)
describe 'back references' do
  it { str =~ /pat/; expect($&).to eq("x") }
  it { str =~ /pat/; expect($`).to eq("x") }
end

# Examples with different numbered references are NOT duplicates
# $1 vs $2 are different in RuboCop AST (nth_ref 1 vs nth_ref 2)
describe 'numbered references' do
  it { str =~ /(a)(b)/; expect($1).to eq("x") }
  it { str =~ /(a)(b)/; expect($2).to eq("x") }
end

# XString (backtick) with different content but same surrounding code NOT duplicates
describe 'xstring diff content' do
  it { result = `cmd1`; expect(result).to eq("x") }
  it { result = `cmd2`; expect(result).to eq("x") }
end

# Method call with empty block {} vs same method call without block are NOT duplicates.
# `any? {}` and `any?` differ in that one passes an empty block, the other does not.
# In RuboCop's AST, (block (send ...) ...) vs (send ...) are different structures.
describe 'empty block vs no block' do
  it "with a block returns false" do
    expect(items.any? {}).to eq(false)
  end

  it "with no block returns false" do
    expect(items.any?).to eq(false)
  end
end

# Same pattern with deeper nesting: with block vs without
describe 'register with and without empty block' do
  it "raises when passed a block" do
    expect do instance.register(:test) {} end.to raise_error(ArgumentError)
  end

  it "raises when no block" do
    expect do instance.register(:test) end.to raise_error(ArgumentError)
  end
end

# Pattern matching: empty array pattern vs empty hash pattern are NOT duplicates
# `value in []` is ArrayPatternNode; `value in {}` is HashPatternNode - different AST
describe 'pattern matching empty array vs hash' do
  it "matches on the empty array" do
    expect(
      (None() in [])
    ).to be(true)
  end

  it "matches on the empty hash" do
    expect(
      (None() in {})
    ).to be(true)
  end
end

# Constant or-write vs class variable or-write are NOT duplicates
# ConstantOrWriteNode (A ||= true) and ClassVariableOrWriteNode (@@a ||= true)
# are structurally different — they must produce different fingerprints
describe "different variable kinds in defined?" do
  it "class variable or-write" do
    defined?(@@a ||= true).should == "assignment"
  end

  it "constant or-write" do
    defined?(A ||= true).should == "assignment"
  end
end

# Lambda params: optional keyword param vs keyword splat are NOT duplicates
# OptionalKeywordParameterNode (b:) vs KeywordRestParameterNode (**b) differ structurally
describe "lambda keyword param vs keyword splat" do
  it "keyword param" do
    f = -> *a, b: { }
    -> { f.ruby2_keywords }.should complain(/Skipping set/)
  end

  it "keyword splat" do
    f = -> *a, **b { }
    -> { f.ruby2_keywords }.should complain(/Skipping set/)
  end
end

# &&/|| operators are NOT duplicates — AndNode vs OrNode must produce different fingerprints
describe 'and vs or operators' do
  it 'uses && operators' do
    defined?(a && b).should == 'expression'
    defined?(b && a).should == 'expression'
  end

  it 'uses || operators' do
    defined?(a || b).should == 'expression'
    defined?(b || a).should == 'expression'
  end
end

# and/or keyword operators are also NOT duplicates (same Prism nodes as &&/||)
describe 'and vs or keyword operators' do
  it 'uses and operator' do
    defined?(x and y).should == 'expression'
  end

  it 'uses or operator' do
    defined?(x or y).should == 'expression'
  end
end

# __FILE__ vs __LINE__ vs __ENCODING__ are NOT duplicates
# SourceFileNode, SourceLineNode, SourceEncodingNode are different leaf types
# that produce zero bytes without type tag emission
describe 'pseudo variables' do
  it { defined?(__FILE__).should == 'expression' }
  it { defined?(__LINE__).should == 'expression' }
  it { defined?(__ENCODING__).should == 'expression' }
end

# __LINE__ is location-sensitive in RuboCop's AST normalization.
# Two otherwise identical examples on different lines are NOT duplicates.
describe 'source line values' do
  it "first line" do
    expect_warning_with_call_site(__FILE__, __LINE__ + 1)
    group.send(shared_method_name, 'name but no block')
  end

  it "second line" do
    expect_warning_with_call_site(__FILE__, __LINE__ + 1)
    group.send(shared_method_name, 'name but no block')
  end
end

# false vs nil are NOT duplicates
# FalseNode and NilNode are different leaf types
describe 'false vs nil literals' do
  it { (not(false)).should be_true }
  it { (not(nil)).should be_true }
end

# true vs false vs nil are all different
describe 'boolean and nil literals' do
  it { expect(defined?(true)).to eq('expression') }
  it { expect(defined?(false)).to eq('expression') }
  it { expect(defined?(nil)).to eq('expression') }
end

# self vs nil are NOT duplicates
describe 'self vs nil' do
  it { expect(self).to be_truthy }
  it { expect(nil).to be_falsey }
end

# Attribute operator write: += vs -= are NOT duplicates
# CallOperatorWriteNode has different operators (+= vs -=)
describe 'attribute operator assignments' do
  it "increments" do
    app.connections += 1
    app.save!
    wait_for(app.connections)
  end

  it "decrements" do
    app.connections -= 1
    app.save!
    wait_for(app.connections)
  end
end

# Different nesting structures: raise inside vs outside inner block
# These produce different ASTs despite having similar source text
describe 'nesting structure differences' do
  it "raise outside inner block" do
    -> do
      IO.open(@fd, "w") do |io|
        mock(io, :close) do
          super()
          record(:called)
        end
        raise RuntimeError
      end
    end.should raise_error(RuntimeError)
  end

  it "raise inside inner block" do
    -> do
      IO.open(@fd, "w") do |io|
        mock(io, :close) do
          super()
          record(:called)
          raise RuntimeError
        end
      end
    end.should raise_error(RuntimeError)
  end
end

# Examples with hash literal spacing differences but same AST are duplicates,
# so the no_offense case is when they have DIFFERENT hash content
describe "hash spacing no offense" do
  it "first" do
    assert_equal({ a: 1, b: 2 }, $result)
  end

  it "second" do
    assert_equal({a: 1, b: 3}, $result)
  end
end

# Nested call argument boundaries are significant in RuboCop's AST.
# `recv(arg).foo(x)` and `recv.foo(arg, x)` are NOT duplicates.
describe 'nested call argument boundaries' do
  it "receiver call takes self" do
    expect(plsql.t_address(@address_attributes).display_address(",")).to eq(@full_address)
  end

  it "outer call takes self" do
    expect(plsql.t_address.display_address(@address_attributes, ",")).to eq(@full_address)
  end
end
