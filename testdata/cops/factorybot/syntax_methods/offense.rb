describe Foo do
  let(:bar) { FactoryBot.create(:bar) }
              ^^^^^^^^^^^^^^^^^ FactoryBot/SyntaxMethods: Use `create` from `FactoryBot::Syntax::Methods`.
end
describe Foo do
  let(:baz) { FactoryBot.build(:baz) }
              ^^^^^^^^^^^^^^^^ FactoryBot/SyntaxMethods: Use `build` from `FactoryBot::Syntax::Methods`.
end
RSpec.describe Foo do
  let(:qux) { FactoryBot.attributes_for(:qux) }
              ^^^^^^^^^^^^^^^^^^^^^^^^^ FactoryBot/SyntaxMethods: Use `attributes_for` from `FactoryBot::Syntax::Methods`.
end
