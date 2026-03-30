describe 'doing x' do
  it "does x" do
  ^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 6.
    expect(foo).to be(bar)
  end

  it "does y" do
  ^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 2.
    expect(foo).to be(bar)
  end
end

describe 'doing y' do
  its(:x) { is_expected.to be_present }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 13.
  its(:x) { is_expected.to be_present }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 12.
end

# Different formatting but same AST body — should still be detected as duplicates
describe 'mixed formatting' do
  it "does x" do
  ^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 21.
    expect(foo).to be(bar)
  end
  it "does y" do expect(foo).to be(bar); end
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 18.
end

# One-liner examples with duplicates
describe 'one-liners' do
  it { is_expected.to be_valid }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 27, 28.
  it { is_expected.to be_valid }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 26, 28.
  it { is_expected.to be_valid }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 26, 27.
end

# Multiline vs single-line with same body
describe 'multiline vs brace' do
  it "multiline" do
  ^^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 36.
    expect(foo).to eq(bar)
  end
  it("single line") { expect(foo).to eq(bar) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 33.
end

# Four duplicates with different descriptions
describe 'four dupes' do
  it "first" do
  ^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 44, 47, 50.
    expect(foo).to be(bar)
  end
  it "second" do
  ^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 41, 47, 50.
    expect(foo).to be(bar)
  end
  it "third" do
  ^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 41, 44, 50.
    expect(foo).to be(bar)
  end
  it "fourth" do
  ^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 41, 44, 47.
    expect(foo).to be(bar)
  end
end

# Examples nested in control flow should still be detected as duplicates
# RuboCop recursively searches for examples, not just direct children
describe 'nested in if' do
  if some_condition
    it "nested a" do
    ^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 63.
      expect(foo).to be(bar)
    end
  else
    it "nested b" do
    ^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 59.
      expect(foo).to be(bar)
    end
  end
end

# Examples with symbol first args (non-string) should still be duplicates
# RuboCop skips the first arg regardless of type when building metadata
describe 'symbol first args' do
  it :pending do
  ^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 75.
    expect(foo).to be(bar)
  end
  it :skipped do
  ^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 72.
    expect(foo).to be(bar)
  end
end

# Integer literal 0 and 00 are the same value (both parse as int 0)
# Parser gem normalizes both to s(:int, 0)
describe 'integer value normalization' do
  it { should cmp 0 }
  ^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 84.
  it { should cmp 00 }
  ^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 83.
end

# Float -0.0 and 0.0 are equal in Ruby (-0.0 == 0.0 is true)
# Parser gem stores both as s(:float, 0.0) since -0.0 == 0.0
describe 'float sign normalization' do
  it "uses 0.0" do
  ^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 94.
    model.value = 0.0
    model.value?.should == false
  end
  it "uses -0.0" do
  ^^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 90.
    model.value = -0.0
    model.value?.should == false
  end
end

# Implicit keyword hash args vs explicit hash args: RuboCop normalizes both
# Parser gem: `foo(a: 1)` and `foo({a: 1})` both produce s(:send, nil, :foo, s(:hash, ...))
describe 'keyword hash vs explicit hash' do
  it "implicit keyword hash" do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 107.
    expect(strategy).to receive(:new).with({ param: "one" })
    cleaner.strategy = [:truncation, param: "one"]
  end
  it "explicit hash" do
  ^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 103.
    expect(strategy).to receive(:new).with({ param: "one" })
    cleaner.strategy = :truncation, { param: "one" }
  end
end

# Examples inside class bodies within before(:context) blocks should be detected.
# RuboCop recursively searches into class bodies for examples.
describe "minitest spec inside before" do
  before(:context) do
    class SomeSpec < Minitest::Spec
      it "does not fail" do
      ^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 122.
      end

      minitest_describe "in context" do
        it "does not fail" do
        ^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 118.
        end
      end
    end
  end
end

# Examples inside singleton class bodies should still be detected.
describe "singleton class wrappers" do
  class << self
    def it_should_redirect_to_show
      it do
      ^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 139.
        is_expected.to redirect_to group_person_path(top_group, top_leader)
      end
    end

    def it_should_redirect_to_index
      it { is_expected.to redirect_to group_person_path(top_group, top_leader) }
      ^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 133.
    end
  end
end

# Assigned example groups with non-RSpec receivers are still searched recursively.
describe "assigned nested example groups" do
  before(:all) do
    @group = Micronaut::Behaviour.describe(Foo, "find these examples") do
      it("I have no options") {}
      ^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 152.
      it("this is awesome", :awesome => true) {}
      ^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 150.
      it("this is too", :awesome => true) {}
      ^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 149.
      it("not so awesome", :awesome => false) {}
      it("I also have no options") {}
      ^ RSpec/RepeatedExample: Don't repeat examples within an example group. Repeated on line(s) 148.
    end
  end
end
