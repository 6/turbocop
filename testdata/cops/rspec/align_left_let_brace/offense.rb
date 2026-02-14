RSpec.describe 'test' do
  let(:foo) { bar }
            ^ RSpec/AlignLeftLetBrace: Align left let brace.
  let(:hi) { baz }
           ^ RSpec/AlignLeftLetBrace: Align left let brace.
  let(:blahblah) { baz }

  let(:long_name) { thing }
  let(:blah) { thing }
             ^ RSpec/AlignLeftLetBrace: Align left let brace.
  let(:a) { thing }
          ^ RSpec/AlignLeftLetBrace: Align left let brace.
end
