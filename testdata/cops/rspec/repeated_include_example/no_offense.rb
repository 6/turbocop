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
