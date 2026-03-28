@some_variable ||= begin
  return some_value if some_condition_is_met
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.

  do_something
end

x = begin
  return 1
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end

@var = begin
  return :foo
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end

# Operator assignments (+=, -=, *=, /=, **=)
some_value = 10

some_value += begin
  return 1 if rand(1..2).odd?
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
  2
end

some_value -= begin
  return 1
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end

some_value *= begin
  return 1
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end

@@class_var += begin
  return 1
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end

$global_var **= begin
  return 1
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end

CONST = begin
  return 1
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end

# And-assignments (&&=)
x = 1
x &&= begin
  return 1
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end

@ivar &&= begin
  return 1
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end

@@cvar &&= begin
  return 1
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end

$gvar &&= begin
  return 1
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end

CONST2 &&= begin
  return 1
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end

# Global variable or-assignment
$gvar ||= begin
  return 1
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end

# Constant or-assignment
CONST3 ||= begin
  return 1
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end

# Constant path and-write / or-write / operator-write
Foo::BAR &&= begin
  return 1
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end

Foo::BAZ ||= begin
  return 1
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end

# Method call assignments
obj = Object.new

obj.foo &&= begin
  return 1
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end

obj.foo ||= begin
  return 1
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end

obj.foo += begin
  return 1
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end

# Index/subscript assignments
arr = [1, 2, 3]

arr[0] &&= begin
  return 1
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end

arr[0] ||= begin
  return 1
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end

arr[0] += begin
  return 1
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end

# Inside a method body (real-world pattern)
def fetch_category
  @category = begin
    Category.new(params)
  rescue ArgumentError => e
    return render json: { errors: [e.message] }
    ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
  end
end

# Inside a class method
class Worker
  def process
    result ||= begin
      return if cancelled?
      ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
      compute_result
    end
  end
end

# Return inside nested def inside begin..end assignment (RuboCop walks into nested defs)
@instance ||= begin
  def instance
    return @instance
    ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
  end
  new
end

# Return inside nested def with rescue inside begin..end assignment
@cached ||= begin
  def helper_method
    return 42 if cached?
    ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
    compute_value
  rescue StandardError
    nil
  end
  MyClass.new
end

# Deeply nested begin inside assignment value (RuboCop's each_node(:kwbegin))
def fetch_data
  status = Timeout.timeout(600) do
    begin
      download
    rescue => e
      return
      ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
    end
  end
end

# Nested def under outer ||= block assignment with explicit begin
@@new_function ||= Puppet::Functions.create_loaded_function(:new, loader) do
  def from_convertible(from, radix)
    case from
    when Integer
      from
    else
      begin
        if from[0] == '0'
          second_char = (from[1] || '').downcase
          if second_char == 'b' || second_char == 'x'
            return Integer(from)
            ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
          end
        end

        Puppet::Pops::Utils.to_n(from)
      rescue TypeError => e
        raise TypeConversionError, e.message
      rescue ArgumentError => e
        match = Patterns::WS_BETWEEN_SIGN_AND_NUMBER.match(from)
        if match
          begin
            return from_args(match[1] + match[2], radix)
            ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
          rescue TypeConversionError
          end
        end
        raise TypeConversionError, e.message
      end
    end
  end
end

# Nested def under outer ||= block assignment with nested rescue begin
@new_function ||= Puppet::Functions.create_loaded_function(:new_float, loader) do
  def from_convertible(from)
    case from
    when Float
      from
    else
      begin
        Float(from)
      rescue TypeError => e
        raise TypeConversionError, e.message
      rescue ArgumentError => e
        match = Patterns::WS_BETWEEN_SIGN_AND_NUMBER.match(from)
        if match
          begin
            return from_args(match[1] + match[2])
            ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
          rescue TypeConversionError
          end
        end
        raise TypeConversionError, e.message
      end
    end
  end
end

# Nested def under assignment value with begin..ensure
@web_mock_http = Class.new do
  def start_without_connect
    if block_given?
      begin
        @socket = Net::HTTP.socket_type.new
        @started = true
        return yield(self)
        ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
      ensure
        do_finish
      end
    end
    @socket = Net::HTTP.socket_type.new
    @started = true
    self
  end
end
