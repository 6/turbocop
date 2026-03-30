# RSpec.describe inside a let block with local variable assignments
# wrapping example calls. The local var assignment means the example
# call doesn't count as a direct child.
context "with an Array" do
  let(:metadata_with_array) do
    meta = nil
    RSpec.describe("group") do
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyExampleGroup: Empty example group detected.
      meta = example('example_with_array', :tag => [:one, 2]).metadata
    end
    meta
  end

  it "matches a symbol" do
    expect(true).to be(true)
  end
end
