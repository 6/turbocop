def foo
  return nil
  ^^^^^^^^^^ Style/ReturnNil: Use `return` instead of `return nil`.
end

def bar
  return nil if something
  ^^^^^^^^^^ Style/ReturnNil: Use `return` instead of `return nil`.
end

def baz
  return nil unless condition
  ^^^^^^^^^^ Style/ReturnNil: Use `return` instead of `return nil`.
end

# lambda do...end creates its own scope — return nil IS flagged
parse = lambda do |field|
  return nil
  ^^^^^^^^^^ Style/ReturnNil: Use `return` instead of `return nil`.
end

# lambda do...end nested inside an outer iterator block — still flagged
items.each do |item|
  handler = lambda do |model|
    return nil unless model.respond_to?(:model_name)
    ^^^^^^^^^^ Style/ReturnNil: Use `return` instead of `return nil`.
  end
end

def method_with_safe_navigation_each(conversation)
  conversation[:messages]&.each do |message|
    return nil unless message[:contents]&.any?
    ^^^^^^^^^^ Style/ReturnNil: Use `return` instead of `return nil`.
  end
end

def set_default_namevar(object)
  object.properties&.each do |property|
    return nil if property.isnamevar
    ^^^^^^^^^^ Style/ReturnNil: Use `return` instead of `return nil`.
  end
end

def try_parse_representation(representation, schema)
  notify_error = proc do |msg|
    yield msg.to_s
    return nil # returns `nil` from the `try_parse_representation` method.
    ^^^^^^^^^^ Style/ReturnNil: Use `return` instead of `return nil`.
  end
end

def method_with_proc
  handler = proc do |result|
    return nil if result.nil?
    ^^^^^^^^^^ Style/ReturnNil: Use `return` instead of `return nil`.
  end
end

def method_with_proc_in_hash
  SomeApi.run(
    handlers: {
      '*' => proc do |result|
        log("error: #{result[:status]}")
        return nil
        ^^^^^^^^^^ Style/ReturnNil: Use `return` instead of `return nil`.
      end
    }
  )
end

def method_with_proc_without_args(acc, literals, literal_re)
  consume_literal = proc do
    acc_str = acc.join
    if acc_str =~ literal_re
      literals << strip_literal(acc_str)
      acc = []
    else
      return nil
      ^^^^^^^^^^ Style/ReturnNil: Use `return` instead of `return nil`.
    end
  end
end
