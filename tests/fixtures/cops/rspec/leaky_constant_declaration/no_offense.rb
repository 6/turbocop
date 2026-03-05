describe SomeClass do
  self::CONSTANT = "Accessible as self.class::CONSTANT".freeze
end
describe SomeClass do
  Foo::CONSTANT = "Accessible as Foo::CONSTANT".freeze
end
describe SomeClass do
  ::CONSTANT = "Accessible as ::CONSTANT".freeze
end
describe SomeClass do
  class self::DummyClass
  end
end
describe SomeClass do
  class Foo::DummyClass
  end
end
describe SomeClass do
  class ::DummyClass
  end
end
describe SomeClass do
  module self::DummyModule
  end
end
describe SomeClass do
  module Foo::DummyModule
  end
end
describe SomeClass do
  module ::DummyModule
  end
end
class DummyClass
end
module DummyModule
end
factory :some_class do
  CONSTANT = "Accessible as ::CONSTANT".freeze
end
describe SomeClass do
  let(:dummy_playbook) do
    Class.new do
      def method
      end
    end
  end
end
