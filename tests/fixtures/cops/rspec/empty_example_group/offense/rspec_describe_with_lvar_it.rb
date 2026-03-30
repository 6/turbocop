# RSpec.describe where the only content is a local variable assignment
# wrapping an `it` call. The lvasgn makes the `it` invisible to
# RuboCop's examples? pattern.
context "due to aggregate_failures" do
  let(:exception) do
    ex = nil
    RSpec.describe do
    ^^^^^^^^^^^^^^^^^ RSpec/EmptyExampleGroup: Empty example group detected.
      ex = it "", :aggregate_failures do
        expect(1).to eq(2)
      end
    end.run
    ex
  end

  it "formats the failure" do
    expect(true).to be(true)
  end
end
