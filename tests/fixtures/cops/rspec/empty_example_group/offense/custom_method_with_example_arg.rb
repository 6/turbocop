# Custom method calls are not recognized as examples/includes.
# Also, `example` used as an argument inside a let block should not
# be treated as an RSpec example method.
describe 'Pincers::Nokogiri::Backend' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyExampleGroup: Empty example group detected.
  let!(:example) { 'some value' }
  let(:pincers) { Pincers.for_nokogiri example }

  it_should_properly_read_the_example
  it_should_support_jquery_selectors
end
