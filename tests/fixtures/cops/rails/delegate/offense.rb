def name
^^^ Rails/Delegate: Use `delegate` to define delegations.
  client.name
end

def email
^^^ Rails/Delegate: Use `delegate` to define delegations.
  account.email
end

def title
^^^ Rails/Delegate: Use `delegate` to define delegations.
  post.title
end

def site_title
^^^ Rails/Delegate: Use `delegate` to define delegations.
  Setting.site_title
end

def [](key)
^^^ Rails/Delegate: Use `delegate` to define delegations.
  @attrs[key]
end

def []=(key, value)
^^^ Rails/Delegate: Use `delegate` to define delegations.
  @attrs[key] = value
end

def fetch(arg)
^^^ Rails/Delegate: Use `delegate` to define delegations.
  client.fetch(arg)
end

def label
^^^ Rails/Delegate: Use `delegate` to define delegations.
  self.class.label
end

# Prefixed delegation: def receiver_method; receiver.method; end
def bar_foo
^^^ Rails/Delegate: Use `delegate` to define delegations.
  bar.foo
end

def client_name
^^^ Rails/Delegate: Use `delegate` to define delegations.
  client.name
end

def config_value(key)
^^^ Rails/Delegate: Use `delegate` to define delegations.
  config.value(key)
end

# Endless method delegations (def foo = bar.foo)
def first = value.first
^^^ Rails/Delegate: Use `delegate` to define delegations.

def last = value.last
^^^ Rails/Delegate: Use `delegate` to define delegations.

def empty? = value.empty?
^^^ Rails/Delegate: Use `delegate` to define delegations.

def size = value.size
^^^ Rails/Delegate: Use `delegate` to define delegations.

def stop = @listener.stop
^^^ Rails/Delegate: Use `delegate` to define delegations.

def root = Engine.root
^^^ Rails/Delegate: Use `delegate` to define delegations.

# Prefixed delegation via self.class receiver: def class_name; self.class.name; end
def class_name
^^^ Rails/Delegate: Use `delegate` to define delegations.
  self.class.name
end

# Direct delegation via self.class (method name matches exactly)
def associations
^^^ Rails/Delegate: Use `delegate` to define delegations.
  self.class.associations
end

def mailer_name
^^^ Rails/Delegate: Use `delegate` to define delegations.
  self.class.mailer_name
end

# Single-line self.class delegation
def keys; self.class.keys; end
^^^ Rails/Delegate: Use `delegate` to define delegations.

# private :method_name with MULTIPLE symbols — RuboCop's pattern only matches single-symbol
# private calls, so private :[]=, :set_element doesn't make []= private for this cop
def []=(i, v)
^^^ Rails/Delegate: Use `delegate` to define delegations.
  @elements[i]= v
end
private :[]=, :set_element, :set_component

# MF declaration in a nested scope should NOT suppress offense at outer level
def check_for_pending_migrations
^^^ Rails/Delegate: Use `delegate` to define delegations.
  Tasks.check_for_pending_migrations
end

# Single-line def methods (inline def...end on one line)
def owner; parser.owner end
^^^ Rails/Delegate: Use `delegate` to define delegations.

def namespace; parser.namespace end
^^^ Rails/Delegate: Use `delegate` to define delegations.

def adapters; Adapter.adapters end
^^^ Rails/Delegate: Use `delegate` to define delegations.

# Methods followed by identifiers containing "module_function" as substring
# should NOT be suppressed — only actual `module_function` calls count
class Handler
  def scope; parser.scope end
  ^^^ Rails/Delegate: Use `delegate` to define delegations.

  def register_module_function(object)
    object.module_function
  end
end

# self.class delegation inside a class/module (nested context)
module Document
  def tag
  ^^^ Rails/Delegate: Use `delegate` to define delegations.
    self.class.tag
  end

  def aliases
  ^^^ Rails/Delegate: Use `delegate` to define delegations.
    self.class.aliases
  end
end

# Private inside a sibling module at the same indent level should NOT suppress
# instance methods outside that module. Pattern from mongomapper: module ClassMethods
# declares `private` at the same indent as the enclosing module's instance methods.
module Plugins
  module Associations
    module ClassMethods
    private
      def create_association(assoc)
        assoc.setup(self)
      end
    end

    def associations
    ^^^ Rails/Delegate: Use `delegate` to define delegations.
      self.class.associations
    end

    def embedded_associations
    ^^^ Rails/Delegate: Use `delegate` to define delegations.
      self.class.embedded_associations
    end
  end
end

# Delegation inside a conditional block after `private` — RuboCop's AST-based sibling
# check doesn't see `private` as affecting defs inside if/else bodies, because the def
# is a child of the if-node, not the class body. Pattern from asciidoctor/timings.rb.
class Timer
  private

  if CONST_DEFINED
    def now
    ^^^ Rails/Delegate: Use `delegate` to define delegations.
      FastClock.now
    end
  else
    def now
    ^^^ Rails/Delegate: Use `delegate` to define delegations.
      ::Time.now
    end
  end
end
