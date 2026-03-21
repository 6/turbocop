RSpec.describe 'test' do
  let(:foo)      { a    }
  let(:hi)       { ab   }
  let(:blahblah) { abcd }

  let(:thing) { ignore_this }
  let(:other) {
    ignore_this_too
  }

  # Comments with let-like text should not be matched
  let(:x) { a }
  # let(:y) { ab }
  let(:z) { abc }

  # let with proc argument (no block) should not be matched
  let(:user, &args[:build_user])

  # Single let should not trigger offense (no group to align with)
  let(:solo) { value }

  # let-like text inside heredoc strings should not be matched
  it 'tests alignment' do
    expect_offense(<<~RUBY)
      let(:foo) { a }
      let(:bar) { abc }
    RUBY
  end
end

# Multi-line calls with single-line blocks must not be treated as single-line lets.
# RuboCop's node.single_line? considers the entire node, not just the block braces.
RSpec.describe 'multi-line call' do
  let('foo') { 1 }
  let('foo' \
      'bar') { 1 }

  let!('foo') { 1 }
  let!('foo' \
      'bar') { 1 }

  let("key#{1}") { 1 }
  let!("key#{1}") { 1 }
end
