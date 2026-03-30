# Examples inside local variable assignments don't count.
# RuboCop's examples? pattern only matches direct children (send, block),
# not lvasgn (local variable assignment) nodes.
describe Rubex, hell: true do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyExampleGroup: Empty example group detected.
  test_case = 'examples'

  examples = ['rcsv', 'array_to_hash'].each do |example|
    context "Case: #{test_case}/#{example}" do
      before do
        @path = example
      end

      it "compiles to C" do
        expect(true).to be(true)
      end
    end
  end
end
