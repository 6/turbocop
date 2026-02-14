describe MyClass do
  include MyClass
          ^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `MyClass`.

  subject { MyClass.do_something }
            ^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `MyClass`.

  before { MyClass.do_something }
           ^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `MyClass`.
end
