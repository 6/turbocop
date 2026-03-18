def some_method
  foo = 1
  puts foo
  1.times do |foo|
              ^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `foo`.
  end
end
def other_method
  foo = 1
  puts foo
  1.times do |i; foo|
                 ^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `foo`.
    puts foo
  end
end
def method_arg(foo)
  1.times do |foo|
              ^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `foo`.
  end
end
# Nested block: inner block param shadows outer block param
def nested_shadow
  items.each do |slug|
    slug.children.map! { |slug| slug.upcase }
                          ^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `slug`.
  end
end
# Destructured block param shadows method arg
def theme_svgs(theme_id)
  sprites.map do |(theme_id, upload_id)|
                   ^^^^^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `theme_id`.
    [theme_id, upload_id]
  end
end
# Block inside if still shadows outer method arg
def some_method(env)
  if some_condition
    pages.each do |env|
                   ^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `env`.
      do_something(env)
    end
  end
end
# Block param shadowing inside if/unless branch still flags
def handler(name)
  if block_given?
    items.each do |name|
                   ^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `name`.
      yield name
    end
  end
end
# Same branch of same if condition node
def some_method
  if condition?
    foo = 1
    puts foo
    bar.each do |foo|
                 ^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `foo`.
    end
  else
    bar.each do |foo|
    end
  end
end
# Splat block param shadows outer
def some_method
  foo = 1
  puts foo
  1.times do |*foo|
              ^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `foo`.
  end
end
# Block block param shadows outer
def some_method
  foo = 1
  puts foo
  proc_taking_block = proc do |&foo|
                               ^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `foo`.
  end
  proc_taking_block.call do
  end
end

# Post parameter shadows in inner block
def configure(*items, tail)
  jobs.each do |tail|
                ^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `tail`.
    puts tail
  end
end

# Keyword rest parameter shadows in inner block
def configure(**options)
  handler = proc do |**options|
                     ^^^^^^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `options`.
    options
  end
  handler.call
end

# FN fix: variable in non-adjacent elsif branches (2+ branches apart)
def magic_method(method)
  if method =~ /^items$/
    items
  elsif method =~ /^first_item$/
    e = find_item(method)
    e ? e[0] : nil
  elsif method =~ /^parent_item$/
    find_parent(method)
  elsif method =~ /^each_item$/
    each_entity(method) { |e| yield e }
                           ^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `e`.
  end
end

# FN fix: variable from while loop, block in else of same if
def compress(body)
  if body.is_a?(::File)
    while part = body.read(8192)
      write(part)
    end
  else
    body.each { |part|
                 ^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `part`.
      write(part)
    }
  end
end

# FN fix: block param shadows outer from nested block in same scope
def build_graph(prev)
  block.prev.each do |prev|
                      ^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `prev`.
    trans[prev]
  end
end

# FN fix: elsif condition assignment, block in later elsif shadows earlier
def validate_archive(archive)
  if archive.too_large?
    report_error
  elsif entry = archive.entries.find { |entry| entry.starts_with?("/") }
    report(entry)
  elsif entry = archive.entries.find { |entry| entry.traversal? }
                                        ^^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `entry`.
    report(entry)
  end
end


# FN fix: variable from block, block param inside block body shadows it
def process_items(times)
  times_by_group.each do |group, times|
                                 ^^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `times`.
    times.each { |t| group.enqueue(t) }
  end
end

# FN fix: variable from method arg, block in else branch shadows it
def handle(response)
  if responses.length == 1
    run(response)
  elsif responses.length > 1
    responses.each_with_index do |response, index|
                                  ^^^^^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `response`.
      say response[:command]
    end
  end
end

