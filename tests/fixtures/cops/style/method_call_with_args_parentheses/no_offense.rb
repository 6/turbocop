# require_parentheses style (default)

# Method calls with parentheses
foo.bar(1, 2)

# No args — not checked
foo.bar

# Operators are exempt
x = 1 + 2

# Setter methods are exempt
foo.bar = baz

# Macros in class body (IgnoreMacros: true by default)
class MyClass
  include Comparable
  extend ActiveSupport
  prepend Enumerable
  attr_reader :name
  belongs_to :user
  has_many :posts
  validates :name, presence: true
  before_action :check_auth
end

# Macros in module body
module MyModule
  include Comparable
  extend ActiveSupport
end

# Top-level receiverless calls are macros too
puts "hello"
require "json"
raise ArgumentError, "bad"
p "debug"
pp object

# Macros inside blocks in class body
class MyClass
  concern do
    bar :baz
  end
end

# Macros inside begin in class body
class MyClass
  begin
    bar :baz
  end
end

# Macros in singleton class
class MyClass
  class << self
    bar :baz
  end
end

# super call with parens (super is not a CallNode)
def foo
  super(a)
end
