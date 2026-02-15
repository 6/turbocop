RSpec.describe Foo do
  it 'uses expect twice' do
  ^^^^^^^^^^^^^^^^^^^^^^ RSpec/MultipleExpectations: Example has too many expectations [2/1].
    expect(foo).to eq(bar)
    expect(baz).to eq(bar)
  end

  it 'uses is_expected twice' do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/MultipleExpectations: Example has too many expectations [2/1].
    is_expected.to receive(:bar)
    is_expected.to receive(:baz)
  end

  it 'uses expect with blocks' do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/MultipleExpectations: Example has too many expectations [2/1].
    expect { something }.to change(Foo, :count)
    expect { other }.to change(Bar, :count)
  end
end

# aggregate_failures: false overrides inherited aggregate_failures
describe Foo, aggregate_failures: true do
  it 'overrides with false', aggregate_failures: false do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/MultipleExpectations: Example has too many expectations [2/1].
    expect(foo).to eq(bar)
    expect(baz).to eq(bar)
  end
end
