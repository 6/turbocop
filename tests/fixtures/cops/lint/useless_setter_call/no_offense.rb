def something
  x = Something.new
  x.attr = 5
  x
end

def another
  @obj.name = 'foo'
end

def third
  obj = Object.new
  obj.name = 'foo'
  do_something(obj)
end

# Setter on method parameter — object persists after method returns
def process(account)
  account.name = 'updated'
end

def inherited(base)
  super
  base.context = context.dup
end

# Variable assigned from non-constructor method call — not a local object
def fetch_and_update
  record = Record.find(1)
  record.name = 'updated'
end

# Variable assigned from shared/factory method — not a local object
def update_shared
  config = Config.current
  config.timeout = 30
end

# Variable contains object passed as argument via intermediate assignment
def process_copy(item)
  @saved = item
  @saved.do_something
  local = @saved
  local.do_something
  local.status = 'done'
end

# Variable assigned from parameter via multiple assignment
def multi_assign(arg)
  _first, target, _third = 1, arg, 3
  target.name = 'updated'
end

# Variable assigned from parameter via logical operator assignment
def logical_assign(arg)
  result = nil
  result ||= arg
  result.name = 'updated'
end

# Setter call on ivar
def update_ivar
  something
  @top.attr = 5
end

# Setter call on cvar
def update_cvar
  something
  @@top.attr = 5
end

# Setter call on gvar
def update_gvar
  something
  $top.attr = 5
end

# Operators ending with = are not setters
def not_a_setter
  top.attr == 5
end

# Exception assignment in begin/rescue (vendor spec)
def with_rescue(bar)
  begin
  rescue StandardError => _
  end
  bar[:baz] = true
end

# Setter not at end of method — not the last expression
def setter_then_return
  x = Something.new
  x.attr = 5
  x
end

# Keyword parameter — object persists after method returns
def with_keyword(record:)
  record.name = 'updated'
end

# Rest parameter — object persists
def with_rest(*items)
  items.size = 0
end

# Block parameter — object persists
def with_block(&callback)
  callback.name = 'updated'
end

# Setter inside method with implicit rescue — not last expression of method
# (the rescue block itself is the last expression, RuboCop doesn't walk into it)
def with_implicit_rescue
  x = Something.new
  x.attr = 5
rescue StandardError
  nil
end

# Setter inside method with implicit ensure — not last expression of method
def with_implicit_ensure
  x = Something.new
  x.attr = 5
ensure
  cleanup
end

# Setter inside method with rescue and else
def with_rescue_else
  x = Something.new
  x.attr = 5
rescue StandardError
  nil
else
  do_something
end
