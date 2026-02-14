include_examples 'examples'
^^^^^^^^^^^^^^^^ RSpec/IncludeExamples: Prefer `it_behaves_like` over `include_examples`.

describe Foo do
  include_examples 'shared stuff'
  ^^^^^^^^^^^^^^^^ RSpec/IncludeExamples: Prefer `it_behaves_like` over `include_examples`.

  context 'nested' do
    include_examples 'more stuff'
    ^^^^^^^^^^^^^^^^ RSpec/IncludeExamples: Prefer `it_behaves_like` over `include_examples`.
  end
end
