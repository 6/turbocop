RSpec.describe User do
  subject { described_class.new }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyLineAfterSubject: Add an empty line after `subject`.
  let(:params) { foo }
end

RSpec.describe Post do
  subject! { described_class.new }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyLineAfterSubject: Add an empty line after `subject!`.
  let(:params) { foo }
end

RSpec.describe Comment do
  subject { described_class.new }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyLineAfterSubject: Add an empty line after `subject`.
  it { is_expected.to be_valid }
end

RSpec.describe Foo do
  subject { described_class.new }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyLineAfterSubject: Add an empty line after `subject`.
  # comment
  describe 'bar' do
  end
end

RSpec.describe Bar do
  subject { described_class.new }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyLineAfterSubject: Add an empty line after `subject`.
  # multiline comment
  # multiline comment
  describe 'bar' do
  end
end
