# Examples inside `for` loops don't count. RuboCop's examples_inside_block?
# only matches (block ...) nodes, not (for ...) nodes.
describe 'for PostgreSQL' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyExampleGroup: Empty example group detected.
  before do
    stub_adapter('PostgreSQL')
  end

  for grouping in [:hour, :day, :week, :month] do
    it "handles #{grouping}" do
      expect(true).to be(true)
    end
  end
end
