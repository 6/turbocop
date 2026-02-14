RSpec.describe User do
  it { is_expected.to be_valid }
  let(:foo) { bar }
  ^^^^^^^^^^^^^^^^^ RSpec/LetBeforeExamples: Move `let` before the examples in the group.
end

RSpec.describe Post do
  context 'nested' do
    it { is_expected.to be_valid }
  end
  let(:baz) { qux }
  ^^^^^^^^^^^^^^^^^^ RSpec/LetBeforeExamples: Move `let` before the examples in the group.
end

RSpec.describe Comment do
  it_behaves_like 'shared stuff'
  let!(:user) { create(:user) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LetBeforeExamples: Move `let!` before the examples in the group.
end
