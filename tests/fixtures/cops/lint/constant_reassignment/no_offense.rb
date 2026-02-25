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
