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

# FN fix: variable in if-branch, block in multi-statement elsif branch
def build_graph
  if items.size == 1
    prev = items.first
    use(prev)
  elsif items.size > 1
    names = items.map(&:name)
    items.each do |prev|
                   ^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `prev`.
      process(prev)
    end
  end
end

# FN fix: variable in case/when, block in different multi-statement when
def run_server(engine)
  case engine
  when "puma"
    server = create_puma
    server.run.join
  when "thin"
    handler = get_handler("thin")
    handler.run(app) do |server|
                         ^^^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `server`.
      server.ssl = true
    end
  end
end

# FN fix: splat rest param inside destructured block param shadows outer
def join_results(fruits)
  actual.map { |(car, *fruits)| [car, fruits.map(&:name)] }
                       ^^^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `fruits`.
end

# FN fix: when-condition assignment in second when clause shadows first when's var
def transform(decls)
  case
  when decl = decls.find {|decl| decl.special? }
    process(decl)
  when decl = decls.find {|decl| decl.lambda? }
                           ^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `decl`.
    transform(decl)
  end
end

# FN fix: variable assigned earlier, block param in find on separate line
def locate(tp, caller_locations)
  loc = build_source_location(tp, caller_locations)
  caller_location = caller_locations
    .find { |loc| loc.path && File.exist?(loc.path) }
             ^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `loc`.
  caller_location
end

# FN fix: multi-assign LHS variable, block in else branch shadows it
def find_source(accounts)
  host, username, password = accounts.find { |h, u, p| h == target }
  if username
    use(host)
  else
    accounts.each do |host, olduser, oldpw|
                      ^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `host`.
      menu.choice(olduser, host)
    end
  end
end

# FN fix: block param shadows variable from outer catch/else scope
def parse_args(sw)
  catch(:prune) do
    visit(:each_option) do |sw|
                            ^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `sw`.
      sw.block.call(arg) if Switch === sw
    end
  end
end

# FN fix: unless/else — block in RHS of assignment in unless body, var also assigned in else
def echo(major, minor)
  unless minor
    item = storage.items.detect do |item|
                                    ^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `item`.
      item.name == major
    end
  else
    item = list.find_item(minor)
  end
end

# FN fix: if/else — block nested in method chain in else body, var in if body
def track_constant(tp, caller_locations)
  if File.exist?(tp.path)
    loc = build_source_location(tp, caller_locations)
  else
    caller_location = caller_locations
      .find { |loc| loc.path && File.exist?(loc.path) }
               ^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `loc`.
    loc = resolve_location(caller_location)
  end
end

# FN fix: variable reassigned inside block scope (case branch), block in else
def parse_in_order(argv, setter)
  opt, arg, sw, val, rest = nil
  catch(:terminate) {
    while arg = argv.shift
      case arg
      when /\A--/
        sw, = complete(:long, opt, true)
      else
        catch(:prune) do
          visit(:each_option) do |sw|
                                  ^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `sw`.
            sw.block.call(arg)
          end
        end
      end
    end
  }
end

# FN fix: adjacent elsif — block nested in method chain, not direct branch child
def schema_example(value)
  if value.key?("oneOf")
    value["oneOf"].first
  elsif value.key?("anyOf")
    ref = value["anyOf"].first
    schema_example(ref)
  elsif value.key?("allOf")
    value["allOf"].map { |ref| schema_example(ref) }.reduce({}, &:merge)
                          ^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `ref`.
  end
end

# FN fix: Thread.new(value) — RuboCop only suppresses Ractor.new, not Thread.new
def threaded_or_sequential(lib, &block)
  if use_threads?
    Thread.new { block.call(lib) }
  else
    value = block.call(lib)
    Thread.new(value) { |value| value }
                         ^^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `value`.
  end
end

# FN fix: Thread.new with splat args — not suppressed (only Ractor.new is special)
def start_thread(*args)
  Thread.new(*args) { |*args| process(*args) }
                       ^^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `args`.
end

# FN fix: reduce with call arg matching block param — not suppressed by RuboCop
def apply_filters(content, filters)
  filters.reduce(content) { |content, filter| filter.apply(content) }
                             ^^^^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `content`.
end

# FN fix: File.open with call arg matching block param
def overwrite_file(file, new_content)
  File.open(file, "w") { |file| file.puts new_content }
                          ^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `file`.
end

# FN fix: inject with call arg matching block param (corpus: elasticsearch-ruby)
def execute(client, test = nil)
  @definition.each.inject(client) do |client, (method_chain, args)|
                                      ^^^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `client`.
    chain = method_chain.split('.')
    client
  end
end

# FN fix: each_with_object with arg matching block param (corpus: locomotivecms)
def extract_exposures(exposures, hash = {}, prefix = nil)
  exposures.each_with_object(hash) do |exposure, hash|
                                                 ^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `hash`.
    key = "#{prefix}#{exposure}"
    hash[key.to_sym] = exposure
  end
end

# FN fix: Dir.chdir with arg matching block param (corpus: foreman)
def mkchdir(dir)
  FileUtils.mkdir_p(dir)
  Dir.chdir(dir) do |dir|
                     ^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `dir`.
    yield(File.expand_path(dir))
  end
end

# FN fix: Find.find with arg matching block param (corpus: fpm)
def remove_compiled_files(path)
  Find.find(path) do |path|
                      ^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `path`.
    FileUtils.rm(path) if path.end_with?('.pyc')
  end
end

# FN fix: custom method with arg matching block param (corpus: ransack)
def initialize(reflection, children, polymorphic_class = nil)
  swapping_reflection_klass(reflection, polymorphic_class) do |reflection|
                                                               ^^^^^^^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `reflection`.
    super(reflection, children)
  end
end

# FN fix: Zip::File.open with arg matching block param (corpus: oxml_xxe)
def read_rels(zipfile, fil_r)
  Zip::File.open(zipfile) do |zipfile|
                              ^^^^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `zipfile`.
    zipfile.read(fil_r)
  end
end

# FN fix: lambda param shadowed by reduce block param (corpus: moneta)
def make_encoder(transforms)
  lambda do |value|
    transforms.reduce(value) do |value, transform|
                                 ^^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `value`.
      transform.encode(value)
    end
  end
end

# FN fix: method param shadowed via with_connection block (corpus: ruby-polars)
def write_database(connection, table_name, if_table_exists)
  with_connection(connection) do |connection|
                                  ^^^^^^^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `connection`.
    connection.table_exists?(table_name)
  end
end

# FN fix: multi-assign var in if-branch, block nested in 2 blocks in multi-stmt else
# (corpus: sup-heliotrope/sup bin/sup-add:61)
# Else branch has multiple statements, so the choose block is NOT if.else_branch.
# Inner each block's parent is choose block ≠ if.else_branch → not suppressed.
def get_login_info_multi(sources, uri)
  unless sources.empty?
    if force_account
      host, username, password = sources.find { |h, u, p| h == target }
      use(host, username, password)
    else
      say "Would you like to use the same account for #{uri}?"
      choose do |menu|
        sources.each do |host, olduser, oldpw|
                         ^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `host`.
          menu.choice(olduser, host)
        end
      end
    end
  end
end

