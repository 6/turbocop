it_should_behave_like 'a foo'
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ItBehavesLike: Prefer `it_behaves_like` over `it_should_behave_like` when including examples in a nested context.

describe Foo do
  it_should_behave_like 'shared stuff'
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ItBehavesLike: Prefer `it_behaves_like` over `it_should_behave_like` when including examples in a nested context.

  context 'nested' do
    it_should_behave_like 'more stuff'
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ItBehavesLike: Prefer `it_behaves_like` over `it_should_behave_like` when including examples in a nested context.
  end
end

# With receiver (vendor pattern uses `_` for any receiver)
@state.it_should_behave_like 'shared behavior'
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ItBehavesLike: Prefer `it_behaves_like` over `it_should_behave_like` when including examples in a nested context.
