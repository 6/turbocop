blah do |i| foo(i)
            ^^^^^^ Layout/MultilineBlockLayout: Block body expression is on the same line as the block start.
  bar(i)
end

blah { |i| foo(i)
           ^^^^^^ Layout/MultilineBlockLayout: Block body expression is on the same line as the block start.
  bar(i)
}

items.each do |x| process(x)
                  ^^^^^^^^^^ Layout/MultilineBlockLayout: Block body expression is on the same line as the block start.
  finalize(x)
end

blah do |i| foo(i)
            ^^^^^^ Layout/MultilineBlockLayout: Block body expression is on the same line as the block start.
  bar(i)
rescue
  nil
end

# Lambda with body on same line as opening brace
html = -> { content
            ^^^^^^^ Layout/MultilineBlockLayout: Block body expression is on the same line as the block start.
  more_content
}

# Lambda with params and body on same line
transform = ->(x) { x + 1
                    ^^^^^ Layout/MultilineBlockLayout: Block body expression is on the same line as the block start.
  y = x * 2
}

# Lambda do..end with body on same line
action = -> do run_task
               ^^^^^^^^ Layout/MultilineBlockLayout: Block body expression is on the same line as the block start.
  cleanup
end

# Lambda with heredoc body on same line as opening brace
render -> { <<~HTML
            ^^^^^^ Layout/MultilineBlockLayout: Block body expression is on the same line as the block start.
<p>hello</p>
HTML
}

# Lambda with method call body on same line
process = -> { transform(data)
               ^^^^^^^^^^^^^^^ Layout/MultilineBlockLayout: Block body expression is on the same line as the block start.
  finalize(data)
}

# Long block args should still be offenses when the joined line is exactly at MaxLineLength
define_deprecated_method_by_hash_args :set_child_packing,
    'child, expand, fill, padding, pack_type',
    'child, :expand => nil, :fill => nil, :padding => nil, :pack_type => nil', 1 do
    |_self, child, expand, fill, padding, pack_type|
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/MultilineBlockLayout: Block argument expression is not on the same line as the block start.
  [child, {:expand => expand, :fill => fill, :padding => padding, :pack_type => pack_type}]
end

define_deprecated_method_by_hash_args :initialize,
    'title, parent, action, back, *buttons',
    ':title => nil, :parent => nil, :action => :open, :buttons => nil' do
    |_self, title, parent, action, back, *buttons|
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/MultilineBlockLayout: Block argument expression is not on the same line as the block start.
  options = {
      :title => title,
      :parent => parent,
      :action => action,
      :buttons => buttons,
  }
  [options]
end

define_deprecated_method_by_hash_args :initialize,
    'image, size = nil',
    ':stock => nil, :icon_name => nil, :icon_set => nil, :icon => nil, :file => nil, :pixbuf => nil, :animation => nil, :surface => nil, :size => nil' do
    |_self, image, size|
    ^^^^^^^^^^^^^^^^^^^ Layout/MultilineBlockLayout: Block argument expression is not on the same line as the block start.
  case image
  when String
    [{:icon_name => image, :size => size}]
  end
end

define_command(:describe_command,
               doc: "Display the documentation of the command.") do
  |name = read_command_name("Describe command: ")|
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/MultilineBlockLayout: Block argument expression is not on the same line as the block start.
  cmd = Commands[name]
end

bindings.instance_eval {
  @xml.xpath("//bindings/mediaType", @namespaces).map {
    |media_type|
    ^^^^^^^^^^^^ Layout/MultilineBlockLayout: Block argument expression is not on the same line as the block start.
    @media_types << MediaType.new(media_type['handler'], media_type['media-type'])
  }
}

foo { |
      ^ Layout/MultilineBlockLayout: Block argument expression is not on the same line as the block start.
;x| }
