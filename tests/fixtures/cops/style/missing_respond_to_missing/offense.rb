class Test1
  def method_missing
  ^^^^^^^^^^^^^^^^^^ Style/MissingRespondToMissing: When using `method_missing`, define `respond_to_missing?`.
  end
end

class Test2
  def self.method_missing
  ^^^^^^^^^^^^^^^^^^^^^^^ Style/MissingRespondToMissing: When using `method_missing`, define `respond_to_missing?`.
  end
end

class Test3
  def self.method_missing
  ^^^^^^^^^^^^^^^^^^^^^^^ Style/MissingRespondToMissing: When using `method_missing`, define `respond_to_missing?`.
  end

  def respond_to_missing?
  end
end

class Test4
  def self.respond_to_missing?
  end

  def method_missing
  ^^^^^^^^^^^^^^^^^^ Style/MissingRespondToMissing: When using `method_missing`, define `respond_to_missing?`.
  end
end

module Test5
  class << self
    def method_missing(method_name, *args, &block)
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MissingRespondToMissing: When using `method_missing`, define `respond_to_missing?`.
    end
  end
end

class Test6
  if RUBY_VERSION < "3"
    def self.method_missing(message, *args, &block)
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MissingRespondToMissing: When using `method_missing`, define `respond_to_missing?`.
    end
  else
    def self.method_missing(message, *args, **kwargs, &block)
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MissingRespondToMissing: When using `method_missing`, define `respond_to_missing?`.
    end
  end
end

class Test7
  def helper
    class_eval do
      def method_missing(method_name, *args, &block)
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MissingRespondToMissing: When using `method_missing`, define `respond_to_missing?`.
      end
    end
  end
end

class Test8
  def self.respond_to_missing?
  end

  class << self
    def method_missing(method_name, *args, &block)
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MissingRespondToMissing: When using `method_missing`, define `respond_to_missing?`.
    end
  end
end

fh = Class.new(Object) do
  def method_missing(*args)
  ^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MissingRespondToMissing: When using `method_missing`, define `respond_to_missing?`.
    raise "failed"
  end
end

obj.instance_eval do
  def method_missing(method, *args)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MissingRespondToMissing: When using `method_missing`, define `respond_to_missing?`.
  end
end

def method_missing(method, *args)
^ Style/MissingRespondToMissing: When using `method_missing`, define `respond_to_missing?`.
  super
end

def method_missing(sym, *args, &block)
^ Style/MissingRespondToMissing: When using `method_missing`, define `respond_to_missing?`.
  super
end

def method_missing(m, *args, &block)
^ Style/MissingRespondToMissing: When using `method_missing`, define `respond_to_missing?`.
  super
end

def method_missing(mhd, *x)
^ Style/MissingRespondToMissing: When using `method_missing`, define `respond_to_missing?`.
  super
end

def method_missing(s, * args, & b)
^ Style/MissingRespondToMissing: When using `method_missing`, define `respond_to_missing?`.
  super
end

object = Object.new

def object.method_missing(selector)
^ Style/MissingRespondToMissing: When using `method_missing`, define `respond_to_missing?`.
  super
end

def method_missing(method, *args, **kwargs, &block)
^ Style/MissingRespondToMissing: When using `method_missing`, define `respond_to_missing?`.
  super
end
