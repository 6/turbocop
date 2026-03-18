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

  # should-style expectations (implicit subject)
  it 'uses should twice' do
  ^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/MultipleExpectations: Example has too many expectations [2/1].
    should eq(1)
    should eq(2)
  end

  it 'uses should_not twice' do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/MultipleExpectations: Example has too many expectations [2/1].
    should_not eq(1)
    should_not eq(2)
  end

  it 'uses are_expected twice' do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/MultipleExpectations: Example has too many expectations [2/1].
    are_expected.to include(1)
    are_expected.to include(2)
  end

  it 'uses should_receive twice' do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/MultipleExpectations: Example has too many expectations [2/1].
    should_receive(:foo)
    should_receive(:bar)
  end

  it 'uses should_not_receive twice' do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/MultipleExpectations: Example has too many expectations [2/1].
    should_not_receive(:foo)
    should_not_receive(:bar)
  end

  it 'mixes expect and should' do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/MultipleExpectations: Example has too many expectations [2/1].
    expect(foo).to eq(bar)
    should eq(1)
  end
end

# focus is a focused example alias (like fit/fspecify)
  focus 'uses expect twice with focus' do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/MultipleExpectations: Example has too many expectations [2/1].
    expect(foo).to eq(bar)
    expect(baz).to eq(bar)
  end
end

# pending used as example group wrapper — nested examples still checked
pending 'deferred feature' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/MultipleExpectations: Example has too many expectations [3/1].
  it 'has too many expectations in pending group' do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/MultipleExpectations: Example has too many expectations [3/1].
    expect(foo).to eq(1)
    expect(bar).to eq(2)
    expect(baz).to eq(3)
  end
end

# skip used as example group wrapper — nested examples still checked
skip 'disabled feature' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/MultipleExpectations: Example has too many expectations [2/1].
  it 'has too many expectations in skip group' do
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/MultipleExpectations: Example has too many expectations [2/1].
    expect(foo).to eq(1)
    expect(bar).to eq(2)
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
