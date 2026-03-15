def name
  client.name.upcase
end

def name(arg)
  client.name
end

def name
  compute_something
end

delegate :name, to: :client

# Class method receivers can't use delegate
def no_replies_scope
  Status.without_replies
end

def find_user
  User.find_by_email(email)
end

# Method name doesn't match delegated method — not a simple delegation
def valid?
  json.present?
end

def cdn_host
  config.asset_host
end

# Safe navigation is ignored
def author_url
  structured_data&.author_url
end

# Argument forwarding with transformation (not simple delegation)
def fetch(key)
  client.fetch(key.to_s)
end

# Argument count mismatch
def [](key, default)
  @attrs[key]
end

# Private methods are ignored — including methods after other method `end`s
# in the same private section
private

def custom_filter
  object.custom_filter
end

def logger
  Rails.logger
end

def discoverable
  Account.discoverable
end

# module_function makes methods private — delegate doesn't apply
module Helpers
  module_function

  def name
    self.name
  end
end

# module_function on a separate line before the method
module Utils
  module_function

  def label
    config.label
  end
end

# inline module_function before def
module Formatters
  module_function def format
    self.format
  end
end

# private :method_name after def — makes the method private
def status
  record.status
end
private :status

# EnforceForPrefixed: false patterns with non-CallNode receivers
# (these are skipped in our prefix check but tested elsewhere)

# Receiver is a method call with arguments — not a simple delegation target
def fox
  bar(42).fox
end

# Method args don't match (param is used as receiver, not forwarded)
def fox(bar)
  bar.fox
end

# module_function :name AFTER the def — RuboCop skips these
module Adapter
  def adapters
    Adapter.adapters
  end; module_function :adapters

  def register(name, condition)
    Adapter.register(name, condition)
  end; module_function :register
end

# module_function :name on a subsequent line in the same module scope
module SecurityUtils
  def secure_compare(a, b)
    OpenSSL.fixed_length_secure_compare(a, b)
  end

  module_function :secure_compare
end

# module_function in ancestor module — class nested inside module
# RuboCop's module_function_declared? checks ALL ancestors, not just immediate scope
module Open4
  module_function :open4
  class SpawnError < StandardError
    def exitstatus
      @status.exitstatus
    end
  end
end

# Endless method (def foo() = expr) — not flagged by RuboCop
class Foo
  def clear() = value.clear
end

# Private with trailing space — `private ` (with trailing space) on its own line
class Bar
  private
  def size
    @range.size
  end
end
