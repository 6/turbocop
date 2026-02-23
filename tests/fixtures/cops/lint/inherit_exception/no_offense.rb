class MyError < StandardError; end
class AnotherError < RuntimeError; end
C = Class.new(StandardError)
class Foo < Bar; end
class Baz; end
D = Class.new(RuntimeError)
