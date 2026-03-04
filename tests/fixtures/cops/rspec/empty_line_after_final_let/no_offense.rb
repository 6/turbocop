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

# Blank line followed by comment before next statement is OK
RSpec.describe CommentCase do
  let(:status) { build(:status) }

  # Set up the environment
  before do
    status.save!
  end
end

# shared_examples_for should NOT be checked (not an example group)
shared_examples_for "a resource with configuration" do
  let(:data) { test_data }
  let(:flags) { test_flags }
  it "works correctly" do
    expect(data).to be_present
  end
end

# shared_examples should NOT be checked
shared_examples "enough reports" do |url|
  let(:report) { build(:abuse_report, url: url) }
  it "can't be submitted" do
    expect(report.save).to be_falsey
  end
end

# shared_context should NOT be checked
shared_context "generate_config is stubbed out" do
  let(:new_config) { "this is a new config!" }
  before { expect(subject).to receive(:generate_config).and_return(new_config) }
end

# Heredoc let with blank line after heredoc terminator — should not fire
# because the blank line check must use the heredoc end, not the let line
RSpec.describe HeredocEdge do
  let(:version_list) { <<~YML }
    2.5.0.beta6: twofivebetasix
    2.5.0.beta4: twofivebetafour
  YML

  it { expect(version_list).to include("twofivebetasix") }
end

# Silly heredoc (<<-) with blank line after terminator
RSpec.describe SillyHeredocEdge do
  let(:scss) { <<-SCSS }
    $foo: (1
        (2 3)
      );
  SCSS

  it { expect(scss).to include("$foo") }
end
