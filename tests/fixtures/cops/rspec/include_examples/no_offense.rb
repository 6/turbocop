it_behaves_like 'examples'

describe Foo do
  it_behaves_like 'shared stuff'

  it_should_behave_like 'examples'

  include_context 'context'
end
