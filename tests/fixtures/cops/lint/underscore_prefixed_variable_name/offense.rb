# Method parameter with underscore prefix that is used
def some_method(_bar)
                ^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
  puts _bar
end

# Optional parameter with underscore prefix that is used
def another_method(_baz = 1)
                   ^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
  puts _baz
end

# Block parameter with underscore prefix that is used
items.each do |_item|
               ^^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
  puts _item
end

# Lambda parameter with underscore prefix that is used
handler = ->(_event) do
             ^^^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
  process(_event)
end

# Local variable assignment with underscore prefix that is used
def process_data
  _result = compute
  ^^^^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
  _result.save
end

# Top-level local variable with underscore prefix that is used
_top = 1
^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
puts _top

# Block-pass parameter with underscore prefix that is used
def invoke_block(&_block)
                  ^^^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
  _block.call
end

# Keyword rest parameter with underscore prefix that is used
def merge_options(**_opts)
                    ^^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
  _opts[:key]
end

# Multi-assignment with underscore prefix that is used
def multi_assign
  _first, _second = compute
  ^^^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
  puts _first
end

# Named capture regex with underscore prefix that is used
def match_name(str)
  /(?<_name>\w+)/ =~ str
  ^^^^^^^^^^^^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
  puts _name
end

# For-loop variable with underscore prefix that is used
def loop_items(items)
  for _item in items
      ^^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
    process(_item)
  end
end

# Block param used inside block body (nested in def with no underscore vars)
def draw(name)
  path = @draw_paths.find do |_path|
                              ^^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
    File.exist?(_path)
  end
end

# Local variable in block body used later in same block
def sync
  items.inject(0) do |sum, field|
    _size = compute(field)
    ^^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
    _size + sum
  end
end

# Bare underscore used as block param
items.each { |_| _ }
              ^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.

# Bare underscore used as method param
def load_data(_)
              ^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
  process(_)
end

# Destructured block param with underscore prefix
children.each { |(_page, _children)| add(_page, _children) }
                  ^^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
                         ^^^^^^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.

# Variable assigned and used inside a block in module body
module HasData
  included do
    _record_name = self.name.sub('Data', '').underscore
    ^^^^^^^^^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
    self.primary_key = "#{_record_name}_id"
  end
end

# Variable assigned at def level, read inside a lambda
def method_with_lambda
  _route = something
  ^^^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
  handler = ->(x) { _route.call(x) }
  handler.call(42)
end

# Variable assigned at def level, read via operator-write inside a lambda
def setup_workspace
  _filenames = nil
  ^^^^^^^^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
  filenames = ->{ _filenames ||= workspace.filenames.to_set }
  filenames.call
end

# Variable assigned and used inside a let block (class-level)
describe 'records' do
  let(:item) do
    _obj = Record.new
    ^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
    _obj.name = 'test'
    _obj.save
    _obj
  end
end

# Variable assigned in module scope, read from a class superclass expression
module SparkMapCommand
  _Base = Spark::Command::Base
  ^^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.

  class Map < _Base
  end
end

module SparkPairCommand
  _Base = Spark::Command::Base
  ^^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.

  class Pair < _Base
  end
end

module SparkSortCommand
  _Base = Spark::Command::Base
  ^^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.

  class SortByKey < _Base
  end
end

module SparkStatisticCommand
  _Base = Spark::Command::Base
  ^^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.

  class Sample < _Base
  end
end

# Variable reassigned and later passed to a method call
def draw(canvas, x, y, w, h, simulation)
  _simulation = simulation
  ^^^^^^^^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
  _simulation = true unless visible?
  process(canvas, x, y, w, h, _simulation)
end

# Block-local variable remains an offense when same name is assigned later in the method
def draw_with_branch(simulation, around_draw_procs)
  if around_draw_procs.empty?
    [1].each do |pro,|
      _simulation = simulation
      ^^^^^^^^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
      _simulation = true
      pro.to_s
      puts _simulation
    end
  else
    _simulation = simulation
  end
end

# Rescue exception capture with underscore prefix that is used
def handle_error
  begin
    risky_operation
  rescue StandardError => _e
                          ^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
    puts _e.message
  end
end

# Pattern match bare underscore used in guard clause
def process_value(value)
  case value
  in _ if _.blank?
     ^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
    "blank"
  end
end

# Pattern match named variable with underscore prefix that is used
def parse_url(parts)
  case parts
  in _, _, "imgaz", "upload", _year_month_day, _
                              ^^^^^^^^^^^^^^^ Lint/UnderscorePrefixedVariableName: Do not use prefix `_` for a variable that is used.
    puts _year_month_day
  end
end
