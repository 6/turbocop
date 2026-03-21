RSpec.describe 'test' do
  let(:foo)      { bar }
  let(:hi)       { baz }
  let(:blahblah) { baz }

  let(:thing) { ignore_this }
  let(:other) {
    ignore_this_too
  }
end

# A single let after a gap (non-let line) is a singleton and never flagged,
# even if its brace column differs from adjacent groups
RSpec.describe 'scoped lets' do
  let(:account) { create(:account) }
  let(:user)    { create(:user) }

  context 'inner scope' do
    let(:reply_mail_without_uuid) { something_long }
    let(:described_subject)       { other }
    let(:email_channel)           { third }
  end
end

# let-like text inside heredocs should NOT be detected as let calls
RSpec.describe 'heredoc' do
  let(:template) { <<~RUBY }
    let(:foo) { bar }
    let(:blahblah) { baz }
  RUBY
end

# let-like text inside strings should NOT be detected
RSpec.describe 'string' do
  let(:code) { "let(:foo) { bar }\nlet(:blahblah) { baz }" }
end

# let with proc argument (no block) should NOT be detected
RSpec.describe 'proc' do
  let(:user, &args[:build_user])
end

# Interpolated string let keys - consecutive lets with brace columns that differ by 1
# due to let vs let! (! adds 1 char) should not produce a false positive when
# the difference is solely due to method name length
RSpec.describe 'interp' do
  let("foo#{1}") { 1 }
  let!("foo#{1}") { 1 }
end

# Multi-line calls with single-line blocks: the call spans multiple lines
# (line continuation with \) but the block braces are on the same line.
# RuboCop's node.single_line? returns false for these — they must NOT be
# included in the single-line lets list, or they shift adjacent chunking.
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
