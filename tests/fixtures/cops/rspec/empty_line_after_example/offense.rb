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

  # Description text containing `end` should not be mistaken for one-line do..end
  it("returns the number of added lines") { is_expected.to eq(1) }
  ^^ RSpec/EmptyLineAfterExample: Add an empty line after `it`.
  it "adds a line to the end" do
    expect(true).to be true
  end

  # rubocop:enable directive — offense should report on the enable directive line
  context 'with enable directive' do
    # rubocop:disable RSpec/Foo
    it 'does this' do
    end
    # rubocop:enable RSpec/Foo
    ^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyLineAfterExample: Add an empty line after `it`.
    it 'does that' do
    end
  end

  # rubocop:enable followed by rubocop:disable
  context 'with enable then disable' do
    # rubocop:disable RSpec/Foo
    it 'does this' do
    end
    # rubocop:enable RSpec/Foo
    ^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyLineAfterExample: Add an empty line after `it`.
    # rubocop:disable RSpec/Foo
    it 'does that' do
    end
    # rubocop:enable RSpec/Foo
  end

  # rubocop:disable directive (not enable) — offense reports on the end line
  context 'with disable directive' do
    it 'does this' do
    end
    ^^^ RSpec/EmptyLineAfterExample: Add an empty line after `it`.
    # rubocop:disable RSpec/Foo
    it 'does that' do
    end
    # rubocop:enable RSpec/Foo
  end
end

# Examples inside a module wrapper — must be detected (RuboCop on_block fires everywhere)
module SomeModule
  RSpec.describe Foo do
    it 'does this' do
    end
    ^^^ RSpec/EmptyLineAfterExample: Add an empty line after `it`.
    it 'does that' do
    end
  end
end

# Examples inside a class wrapper
class SomeClass
  RSpec.describe Bar do
    it 'first example' do
    end
    ^^^ RSpec/EmptyLineAfterExample: Add an empty line after `it`.
    it 'second example' do
    end
  end
end
