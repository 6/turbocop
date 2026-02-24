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
end
