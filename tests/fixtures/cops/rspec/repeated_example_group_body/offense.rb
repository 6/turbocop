describe 'doing x' do
^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated describe block body on line(s) [5]
  it { cool_predicate_method }
end

describe 'doing y' do
^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated describe block body on line(s) [1]
  it { cool_predicate_method }
end

context 'when awesome case' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated context block body on line(s) [13]
  it { another_predicate_method }
end

context 'when another awesome case' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated context block body on line(s) [9]
  it { another_predicate_method }
end

describe 'quoting case a' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated describe block body on line(s) [21]
  it { expect(subject).to eq('hello') }
end

describe 'quoting case b' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated describe block body on line(s) [17]
  it { expect(subject).to eq("hello") }
end

context 'parens case a' do
^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated context block body on line(s) [29]
  it { expect(subject).to eq(1) }
end

context 'parens case b' do
^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated context block body on line(s) [25]
  it { expect(subject).to eq 1 }
end

control 'test-01' do
  describe 'first check' do
  ^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated describe block body on line(s) [37]
    it { should eq 0 }
  end
  describe 'second check' do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated describe block body on line(s) [34]
    it { should eq 0 }
  end
end

if condition
  describe 'branch a' do
  ^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated describe block body on line(s) [47]
    it { should exist }
    it { should be_enabled }
  end
  describe 'branch b' do
  ^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated describe block body on line(s) [43]
    it { should exist }
    it { should be_enabled }
  end
elsif other_condition
  describe 'branch c' do
  ^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated describe block body on line(s) [56]
    it { should be_valid }
    it { should respond_to :name }
  end
  describe 'branch d' do
  ^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated describe block body on line(s) [52]
    it { should be_valid }
    it { should respond_to :name }
  end
else
  describe 'branch e' do
  ^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated describe block body on line(s) [64]
    it { should be_nil }
  end
  describe 'branch f' do
  ^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated describe block body on line(s) [61]
    it { should be_nil }
  end
end

# Negative zero vs zero — Parser gem folds -0.0 into float literal where -0.0 == 0.0
RSpec.describe 'Float#negative?' do
  describe 'on zero' do
  ^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated describe block body on line(s) [75]
    it { 0.0.negative?.should be_false }
  end

  describe 'on negative zero' do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated describe block body on line(s) [71]
    it { -0.0.negative?.should be_false }
  end
end

# Empty block params || vs no params — Parser gem treats both as empty args
RSpec.describe 'blocks' do
  describe 'taking zero arguments' do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated describe block body on line(s) [86]
    it { @y.z { 1 }.should == 1 }
  end

  describe 'taking || arguments' do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated describe block body on line(s) [82]
    it { @y.z { || 1 }.should == 1 }
  end
end

module Macros
  def it_should_properly_read_the_example
    describe "[]" do
    ^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated describe block body on line(s) [101]
      it { expect(pincers.search('p.description').classes).to eq(['history', 'description']) }
    end

    describe "text" do
      it { expect(pincers.text).to include('Lorem ipsum dolor sit amet') }
    end

    describe "classes" do
    ^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedExampleGroupBody: Repeated describe block body on line(s) [93]
      it { expect(pincers.search('p.description').classes).to eq(['history', 'description']) }
    end
  end
end
