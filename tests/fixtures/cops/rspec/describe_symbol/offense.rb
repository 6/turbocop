describe(:some_method) { }
         ^^^^^^^^^^^^ RSpec/DescribeSymbol: Avoid describing symbols.

describe(:some_method, "description") { }
         ^^^^^^^^^^^^ RSpec/DescribeSymbol: Avoid describing symbols.

RSpec.describe Foo do
  describe :to_s do
           ^^^^^ RSpec/DescribeSymbol: Avoid describing symbols.
  end
end
