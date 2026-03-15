X = :foo
Y = :bar
Z = :baz
W = 1
V = 'hello'
X ||= :default

# Constants in begin/rescue (fallback pattern)
begin
  ADAPTER = File.read(filename)
rescue
  ADAPTER = nil
end

# Constants in if/else branches
if something
  MODE = :fast
else
  MODE = :slow
end

# Constants with if modifier
LIMIT = :default
LIMIT = :override unless something

# Constants in if/else inside a class
class Config
  TIMEOUT = :default

  if something
    TIMEOUT = :fast
    TIMEOUT = :turbo
  else
    TIMEOUT = :slow
  end
end

# Constants in if without else
class Settings
  if something
    LEVEL = :high
  end
end

# Constants inside a block
class Runner
  LABEL = :original

  silence_warnings do
    LABEL = :updated
  end
end

# Constants inside a block with multiple statements
class Processor
  TAG = :old

  silence_warnings do
    TAG = :new
    EXTRA = :added
  end
end

# Constants in Class.new block
MARKER = :first

Class.new do
  MARKER = :second
end

# Variable-path constant assignment (not a fixed path)
lvar::FOO = 1
lvar::FOO = 2

# Nested variable-path constant
lvar::FOO::BAR = 1
lvar::FOO::BAR = 2

# remove_const clears the constant so re-assignment is OK
class Cleaner
  TOKEN = :old

  remove_const :TOKEN

  TOKEN = :new
end

# remove_const with string argument
class Fixer
  STAMP = :old

  remove_const 'STAMP'

  STAMP = :new
end

# Constant in namespace and top-level :: prefix
module Scope
  INNER = :yes
  ::INNER = :no
end

# Same name but different namespace paths from top-level
module Outer
  module Inner
    DEEP = :a
  end

  ::Inner::DEEP = :b
end

# self.remove_const also clears the constant
class Modifier
  FLAG = :old

  self.remove_const :FLAG

  FLAG = :new
end

# Constant assignment inside a method (not simple)
class Worker
  MARK = :original

  def setup
    MARK = :changed
  end
end

# Constant assignment inside a lambda (not simple)
class Evaluator
  LABEL = :static

  process = -> {
    LABEL = :dynamic
  }
end

# Explicit begin blocks are not simple assignment contexts
FOO = :bar

begin
  FOO = :baz
end

# Singleton class bodies are ignored by RuboCop for this cop
class SingletonConstants
  class << self
    FLAG = :one
    FLAG = :two
  end

  FLAG = :three
end
