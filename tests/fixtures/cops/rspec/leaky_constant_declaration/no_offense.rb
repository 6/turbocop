describe SomeClass do
  self::CONSTANT = "Accessible as self.class::CONSTANT".freeze
end
describe SomeClass do
  Foo::CONSTANT = "Accessible as Foo::CONSTANT".freeze
end
describe SomeClass do
  class self::DummyClass
  end
end
describe SomeClass do
  module self::DummyModule
  end
end
class DummyClass
end
