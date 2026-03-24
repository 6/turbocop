describe Foo do
  let!(:foo) { bar }
  ^^^^^^^^^^ RSpec/LetSetup: Do not use `let!` to setup objects not referenced in tests.

  it 'does not use foo' do
    expect(baz).to eq(qux)
  end
end

describe Foo do
  context 'when something special happens' do
    let!(:foo) { bar }
    ^^^^^^^^^^ RSpec/LetSetup: Do not use `let!` to setup objects not referenced in tests.

    it 'does not use foo' do
      expect(baz).to eq(qux)
    end
  end

  it 'references some other foo' do
    foo
  end
end

describe Foo do
  let!(:bar) { baz }
  ^^^^^^^^^^ RSpec/LetSetup: Do not use `let!` to setup objects not referenced in tests.
end

# include_examples block with unused let!
describe Widget do
  include_examples 'shared behavior' do
    let!(:item) { create(:item) }
    ^^^^^^^^^^ RSpec/LetSetup: Do not use `let!` to setup objects not referenced in tests.

    it 'works' do
      expect(true).to be true
    end
  end
end

# include_context block with unused let!
describe Widget do
  include_context 'with setup' do
    let!(:record) { create(:record) }
    ^^^^^^^^^^ RSpec/LetSetup: Do not use `let!` to setup objects not referenced in tests.

    it 'works' do
      expect(true).to be true
    end
  end
end

# RSpec.describe with unused let!
RSpec.describe Widget do
  let!(:item) { create(:item) }
  ^^^^^^^^^^ RSpec/LetSetup: Do not use `let!` to setup objects not referenced in tests.

  it 'does not use item' do
    expect(true).to be true
  end
end

# Multi-line let! with local variable shadowing the let! name
# The let! body assigns to a local variable with the same name,
# but that internal reference should not count as "used"
describe Widget do
  let!(:order) do
  ^^^^^^^^^^ RSpec/LetSetup: Do not use `let!` to setup objects not referenced in tests.
    order = create(:order, user: user)
    order.items << items
    order.save!
    order
  end

  it 'checks count' do
    expect(Order.count).to eq(1)
  end
end

# let! inside a non-example-group block (e.g., iterator)
# RuboCop's ExampleGroup#lets recurses through non-scope-change blocks
describe Widget do
  [1, 2].each do |i|
    let!(:record) { create(:record, position: i) }
    ^^^^^^^^^^ RSpec/LetSetup: Do not use `let!` to setup objects not referenced in tests.

    it 'does not use record' do
      expect(Record.count).to eq(2)
    end
  end
end

# let! with trailing if modifier — should still be detected
describe Widget do
  let!(:projects) { create_list(:project, 3) } if config[:projects]
  ^^^^^^^^^^ RSpec/LetSetup: Do not use `let!` to setup objects not referenced in tests.

  it 'does something' do
    expect(true).to be true
  end
end

# let! with trailing unless modifier — should still be detected
describe Widget do
  let!(:record) { create(:record) } unless skip_setup
  ^^^^^^^^^^ RSpec/LetSetup: Do not use `let!` to setup objects not referenced in tests.

  it 'does something' do
    expect(true).to be true
  end
end

# let! inside an else branch should still be detected
describe EmbargoService do
  if valkyrie_enabled?
    it 'uses the valkyrie path' do
      expect(true).to be true
    end
  else
    let!(:work_with_released_embargo) { create(:embargoed_work) }
    let!(:work_with_embargo_in_effect) { create(:embargoed_work) }
    let!(:work_without_embargo) { create(:generic_work) }
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/LetSetup: Do not use `let!` to setup objects not referenced in tests.

    it 'uses the embargoed works' do
      expect(work_with_released_embargo).to be_present
      expect(work_with_embargo_in_effect).to be_present
    end
  end
end
