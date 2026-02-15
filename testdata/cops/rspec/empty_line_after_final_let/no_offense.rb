RSpec.describe User do
  let(:a) { a }
  let(:b) { b }

  it { expect(a).to eq(b) }
end

RSpec.describe Post do
  let(:x) { 1 }
  let(:y) { 2 }
end

RSpec.describe Comment do
  let(:foo) { 'bar' }

  specify { expect(foo).to eq('bar') }
end

::RSpec.describe Widget do
  let(:w) { Widget.new }

  it { expect(w).to be_valid }
end

# Heredoc in let — the blank line should be detected correctly
RSpec.describe HeredocCase do
  let(:template) { <<~TEXT }
    some long text here
    and more text
  TEXT

  it { expect(template).to include('some') }
end
