class MyError < StandardError; end
class AnotherError < RuntimeError; end
C = Class.new(StandardError)
class Foo < Bar; end
class Baz; end
D = Class.new(RuntimeError)

# Qualified constant path ending in Exception (not top-level Exception)
class CustomError < Foreman::Exception; end
class AnotherError < ::Foreman::Exception; end
class NestedError < MyApp::Errors::Exception; end
