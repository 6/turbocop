RSpec.describe Foo do
  it 'does this' do
  end
  ^^^ RSpec/EmptyLineAfterExample: Add an empty line after `it`.
  it 'does that' do
  end

  specify do
  end
  ^^^ RSpec/EmptyLineAfterExample: Add an empty line after `specify`.
  specify 'something else' do
  end

  it 'another example' do
  end
  ^^^ RSpec/EmptyLineAfterExample: Add an empty line after `it`.
  # a comment
  it 'yet another' do
  end

  # One-liner followed by multi-liner should fire
  it("returns false") { expect(true).to be false }
  ^^ RSpec/EmptyLineAfterExample: Add an empty line after `it`.
  it "adds the errors" do
    expect(true).to be true
  end
end
