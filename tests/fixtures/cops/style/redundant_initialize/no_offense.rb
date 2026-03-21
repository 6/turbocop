def initialize
  # initializer comment
end

def initialize(a, b)
  do_something
end

def initialize(a, b)
  super
  do_something
end

def do_something
end

def initialize(a, b)
  super()
end

def initialize(a, b = 5)
  super
end

def initialize(*args)
  super
end

def initialize(**kwargs)
  super
end

# Empty initialize with parameter — not redundant (overrides parent)
def initialize(_assistant); end
def initialize(arg)
end

# Inline comment on def line — allowed with AllowComments: true (default)
def initialize # some comment
  super
end

# super with different number of args — not redundant
def initialize(a, b)
  super(a)
end

# super with different arg names — not redundant
def initialize(a, b)
  super(b, a)
end

# super with extra args — not redundant
def initialize(a)
  super(a, b)
end

# super with a block (do...end) — not redundant, block adds behavior
def initialize(base, target, association)
  super do
    bind_one
  end
end

# super with a block (curly braces) — not redundant
def initialize
  super() { |h, k| h[k] = [] }
end

# bare super with a block — not redundant
def initialize
  super do
    1
  end
end

# super(args) with a block — not redundant
def initialize(version)
  super(version) do
    setup
  end
end

# RuboCop's contains_comments? includes comments between end and next sibling.
# Empty initialize followed by a comment before next code — not redundant per RuboCop.
class MockController
  def initialize
  end

  # Expected controller has no cookies method.
  def cookie_domain
    nil
  end
end

# super-forwarding initialize followed by a comment before next sibling
class Base
  def initialize(page, handler)
    super
  end

  # OPTIONAL
  #
  # Setup method for subclasses.
  def setup; end
end

# Single-line def with inline comment after end — not redundant (loofah pattern)
def initialize; end # rubocop:disable Lint/MissingSuper

# super(args) with trailing blank lines (twilio-ruby pattern)
class Resource
  # @param [Version] version Version that contains the resource
  # @return [ResourceInstance] ResourceInstance
  def initialize(version)
    super(version)


  end

  # Additional methods follow
  def fetch; end
end
