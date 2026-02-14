RSpec.describe 'test' do
  let(:foo)      { a }
                     ^ RSpec/AlignRightLetBrace: Align right let brace.
  let(:hi)       { ab }
                      ^ RSpec/AlignRightLetBrace: Align right let brace.
  let(:blahblah) { abcd }

  let(:blahblah) { a }
                     ^ RSpec/AlignRightLetBrace: Align right let brace.
  let(:blah)     { bc }
                      ^ RSpec/AlignRightLetBrace: Align right let brace.
  let(:a)        { abc }
end
