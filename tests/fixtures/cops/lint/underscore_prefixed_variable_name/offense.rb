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
