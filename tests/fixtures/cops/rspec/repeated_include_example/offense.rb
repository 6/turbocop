describe 'foo' do
  include_examples 'an x'
  ^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedIncludeExample: Repeated include of shared_examples 'an x' on line(s) [4]
  include_examples 'something else'
  include_examples 'an x'
  ^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedIncludeExample: Repeated include of shared_examples 'an x' on line(s) [2]
end

describe 'bar' do
  it_behaves_like 'an x'
  ^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedIncludeExample: Repeated include of shared_examples 'an x' on line(s) [9]
  it_behaves_like 'an x'
  ^^^^^^^^^^^^^^^^^^^^^^ RSpec/RepeatedIncludeExample: Repeated include of shared_examples 'an x' on line(s) [8]
end
