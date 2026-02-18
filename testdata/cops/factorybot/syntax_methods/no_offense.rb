RSpec.describe Foo do
  let(:bar) { create(:bar) }
end
RSpec.describe Foo do
  let(:baz) { build(:baz) }
end
describe Foo do
  let(:qux) { attributes_for(:qux) }
end
