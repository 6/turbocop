RSpec.describe Foo do
  it 'does this' do
  end

  it 'does that' do
  end

  it { one }
  it { two }

  specify do
  end

  # Comment with blank line between it and next example is OK
  it 'does something' do
  end
  # rubocop:enable RSpec/SomeOtherCop

  it 'another thing' do
  end

  # Bare example calls without blocks are NOT example declarations
  # (e.g. `scenario` from let(:scenario), `skip(...)` inside before)
  before do
    skip('not configured') unless configured?
  end

  let(:scenario) { create(:scenario) }

  it 'uses scenario' do
    allow(obj).to receive(:items).and_return([scenario])
    expect(scenario).to be_truthy
  end

  # Example inside if/else — last child before `else` needs no blank line
  [true, false].each do |flag|
    if flag
      it 'does one thing' do
        expect(flag).to be true
      end
    else
      it 'does another thing' do
        expect(flag).to be false
      end
    end
  end

  # One-liner followed by comment then another one-liner
  it { is_expected.to validate_presence_of(:name) }
  # it { is_expected.to validate_uniqueness_of(:code) }
  it { is_expected.to belong_to(:account) }
  it { is_expected.to have_one(:inbox) }

  # Example followed by comment then `end`
  context 'nested' do
    it 'works' do
    end
    # rubocop:enable RSpec/AnyInstance
  end

  # One-liner before a brace terminator should be treated as last child
  context {
    it { should == 0 }
  }

  # Consecutive `its` one-liners should be allowed
  describe '#attributes' do
    subject { record }
    its(:name) { should eq 'test' }
    its(:status) { should eq 'active' }
    its(:role) { should eq 'admin' }
  end

  # Consecutive `xit` one-liners (skipped examples)
  xit { expect(1).to eq(1) }
  xit { expect(2).to eq(2) }

  # Consecutive `fit` one-liners (focused examples)
  fit { expect(1).to eq(1) }
  fit { expect(2).to eq(2) }

  # Consecutive `pending` one-liners
  pending { expect(1).to eq(1) }
  pending { expect(2).to eq(2) }

  # Mixed `its` and `it` consecutive one-liners
  its(:name) { should be_present }
  it { is_expected.to validate_presence_of(:name) }
  its(:role) { should eq 'user' }

  # Heredoc inside example — heredoc terminator extends past the block end
  # so the cop must account for heredoc extent when computing the end line
  context 'with heredoc' do
    it { should == normalize_indent(<<-OUT) }
      some content here
    OUT

    it 'does something else' do
      expect(true).to be true
    end
  end

  # Heredoc with squiggly syntax
  context 'with squiggly heredoc' do
    it 'renders output' do
      expect(result).to eq(<<~EXPECTED)
        line one
        line two
      EXPECTED
    end

    it 'does another thing' do
      expect(true).to be true
    end
  end

  # Whitespace-only separator lines should count as blank.
  it 'handles whitespace separator' do
    expect(true).to be true
  end

  it 'next example after whitespace separator' do
    expect(true).to be true
  end

  # Consecutive do;...;end; one-liners with trailing semicolons
  context 'with trailing semicolons' do
    it "a == b" do; expect(true).to eq(true); end;
    it "a < b"  do; expect(true).to eq(true); end;
    it "a > b"  do; expect(true).to eq(true); end;
  end

  # Example nested as the only/last child inside a wrapper block on the same line
  # (e.g., let_context, shared_examples_for, etc.)
  describe 'wrapping' do
    wrapper(option: 'a') { it { should eq(1) } }
    wrapper(option: 'b') { it { should eq(2) } }
    wrapper(option: 'c') { it { should eq(3) } }
  end

  # Example is last child inside a parent block — extra content after
  # the example on the same line (closing braces of parent)
  context 'nested blocks' do
    [1, 2].each { |n| it { expect(n).to be_positive } }
  end
end
