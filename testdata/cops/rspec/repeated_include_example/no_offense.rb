describe 'foo' do
  include_examples 'an x'
  include_examples 'a y'
end

describe 'bar' do
  it_behaves_like 'an x'
end

describe 'baz' do
  it_behaves_like 'an x'
end

describe 'heredoc args with same name but different content' do
  it_behaves_like 'misaligned', <<~RUBY, false
    puts 1; class Test
      end
  RUBY

  it_behaves_like 'misaligned', <<~RUBY, false
    var =
      if test
    end
  RUBY
end
