def some_method
  foo = 1
  puts foo
  1.times do |bar|
  end
end
def some_method
  foo = 1
  puts foo
  1.times do
    foo = 2
  end
end
def some_method
  _ = 1
  puts _
  1.times do |_|
  end
end
def some_method
  _foo = 1
  puts _foo
  1.times do |_foo|
  end
end

# Variables from sibling blocks should not be treated as outer locals
def sibling_blocks
  [1].each { |x| y = x + 1; puts y }
  [2].each { |y| puts y }
end

# Variables from sibling lambdas should not leak
def sibling_lambdas
  a = lambda { |n| n = n.to_s; puts n }
  b = lambda { |n| puts n }
  a.call(1)
  b.call(2)
end

# Variables defined inside a block should not shadow in sibling block
class MyClass
  scope :secured, ->(guardian) { ids = guardian.secure_ids; puts ids }
  scope :with_parents, ->(ids) { where(ids) }
end

# Nested block variables should not leak to outer scope
def nested_blocks
  items.each do |item|
    item.children.each { |child| value = child.name; puts value }
  end
  other_items.each do |other|
    other.parts.each { |value| puts value }
  end
end

# Different branches of case/when - not flagged
def different_branches
  case filter
  when "likes-min"
    value = values.last
    value if value =~ /\A\d+\z/
  when "order"
    values.flat_map { |value| value.split(",") }
  end
end

# Variable used in declaration of outer — block is the RHS of the assignment
def some_method
  foo = bar { |foo| baz(foo) }
end

# Variable used in return value assignment of if
def some_method
  foo = if condition
          bar { |foo| baz(foo) }
        end
end

# Different branches of if condition
def some_method
  if condition?
    foo = 1
  elsif other_condition?
    bar.each do |foo|
    end
  else
    bar.each do |foo|
    end
  end
end

# Different branches of unless condition
def some_method
  unless condition?
    foo = 1
  else
    bar.each do |foo|
    end
  end
end

# Different branches of if condition in a nested node
def some_method
  if condition?
    foo = 1
  else
    bar = [1, 2, 3]
    bar.each do |foo|
    end
  end
end

# Different branches of case condition
def some_method
  case condition
  when foo then
    foo = 1
  else
    bar.each do |foo|
    end
  end
end

# Sibling block variables (from prior block body) don't shadow
def x(array)
  array.each { |foo|
    bar = foo
  }.each { |bar|
  }
end

# Class-level begin block vars don't shadow method-level block params
class MyTranslator
  MAPPING =
    begin
      from = "abc"
      to = "xyz"
      from.chars.zip(to.chars)
    end

  def translate(name)
    MAPPING.each { |from, to| name.gsub!(from, to) }
    name
  end
end

# Later method params are not visible in earlier default lambdas
def build_handlers(
  outer: ->(cursor) { cursor },
  inner: ->(item, cursor) { [item, cursor] },
  cursor: nil
)
  [outer, inner, cursor]
end

# Top-level locals do not leak into class body proc scopes
command = "outer"

class Worker
  HANDLER = proc do |command|
    puts command
  end
end

# Ractor.new block — shadowing is intentional (Ractor can't access outer scope)
def start_ractor(*args)
  Ractor.new(*args) do |*args|
    puts args.inspect
  end
end

# Ractor.new with single param
def start_worker(p)
  Ractor.new(p) do |p|
    puts p.inspect
  end
end

# Variable assigned in when condition, block param in when body
def process(env)
  case
  when decl = env.fetch(:type, nil)
    decl.each do |decl|
      puts decl
    end
  when decl = env.fetch(:other, nil)
    decl.map do |decl|
      decl.to_s
    end
  end
end

# FP fix: variable assigned in elsif condition, block in different elsif body
def parse_input(params)
  if msgpack = params['msgpack']
    parse_msgpack(msgpack)
  elsif js = params['json']
    parse_json(js)
  elsif ndjson = params['ndjson']
    ndjson.split(/\r?\n/).each do |js|
      parse_json(js)
    end
  end
end

# FP fix: variable assigned in if condition, block in else branch
def find_account(email)
  if a = lookup(email)
    a
  else
    regexen.argfind { |re, a| re =~ email && a }
  end
end

# FP fix: variable assigned in case predicate, block in when body
def format_value(key, opts)
  case value = send(key)
  when String then "#{opt_key(key, opts)}=#{value.inspect}"
  when Array  then value.map { |value| "#{opt_key(key, opts)}=#{value.inspect}" }
  else opt_key(key, opts)
  end
end


# FP fix: variable assigned in one when body, block param in different when body
def process_slug(slug)
  case slug
  when 'items'
    node = find_node('#Items')
    node.css('li').each { |n| register(n) }
  when 'utils'
    css('dl > dt').each do |node|
      register(node)
    end
  end
end

# FP fix: 3 when branches - block param in one, local var in another, block param in third
def process_data(slug)
  case slug
  when 'first'
    css('h2').each do |heading|
      heading.css('a').each do |node|
        register(node)
      end
    end
  when 'second'
    node = find('#Section')
    node = node.next while node.name != 'ul'
    node.css('li').each do |n|
      register(n)
    end
  when 'third'
    css('dl > dt').each do |node|
      register(node)
    end
  end
end

