def process
  object.save
         ^^^^ Rails/SaveBang: Use `save!` instead of `save` if the return value is not checked.
  object.save(name: 'Tom', age: 20)
         ^^^^ Rails/SaveBang: Use `save!` instead of `save` if the return value is not checked.
  object.update(name: 'Tom', age: 20)
         ^^^^^^ Rails/SaveBang: Use `update!` instead of `update` if the return value is not checked.
  save
  ^^^^ Rails/SaveBang: Use `save!` instead of `save` if the return value is not checked.
  nil
end

# CREATE methods in local variable assignments should be flagged (return value not checked with persisted?)
def create_examples
  x = object.create
             ^^^^^^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked. Or check `persisted?` on model returned from `create`.
  y = object.find_or_create_by(name: 'Tom')
             ^^^^^^^^^^^^^^^^^^ Rails/SaveBang: Use `find_or_create_by!` instead of `find_or_create_by` if the return value is not checked. Or check `persisted?` on model returned from `find_or_create_by`.
  nil
end

# CREATE methods in conditions should get conditional message
if object.create
          ^^^^^^ Rails/SaveBang: `create` returns a model which is always truthy.
  puts "created"
end

unless object.create
              ^^^^^^ Rails/SaveBang: `create` returns a model which is always truthy.
  puts "not created"
end

# CREATE method in boolean expression
object.create && notify_user
       ^^^^^^ Rails/SaveBang: `create` returns a model which is always truthy.
object.create || raise("failed")
       ^^^^^^ Rails/SaveBang: `create` returns a model which is always truthy.

# Persist call in body of modifier-if (void context, not the condition)
object.save if false
       ^^^^ Rails/SaveBang: Use `save!` instead of `save` if the return value is not checked.

# Persist call in else branch
if condition
  puts "true"
else
  object.save
         ^^^^ Rails/SaveBang: Use `save!` instead of `save` if the return value is not checked.
end

# Safe navigation calls
object&.save
        ^^^^ Rails/SaveBang: Use `save!` instead of `save` if the return value is not checked.
object&.update(name: 'Tom')
        ^^^^^^ Rails/SaveBang: Use `update!` instead of `update` if the return value is not checked.

# Variable arguments
object.save(variable)
       ^^^^ Rails/SaveBang: Use `save!` instead of `save` if the return value is not checked.
object.save(*variable)
       ^^^^ Rails/SaveBang: Use `save!` instead of `save` if the return value is not checked.
object.save(**variable)
       ^^^^ Rails/SaveBang: Use `save!` instead of `save` if the return value is not checked.

# CREATE in case statement condition
case object.create
            ^^^^^^ Rails/SaveBang: `create` returns a model which is always truthy.
when true
  puts "true"
end

# Persist calls inside blocks (void context within block body)
records.map do |r|
  r.update(name: 'Tom')
    ^^^^^^ Rails/SaveBang: Use `update!` instead of `update` if the return value is not checked.
  nil
end

# Persist calls inside nested blocks
items.each do |i|
  i.records.each do |r|
    r.save
      ^^^^ Rails/SaveBang: Use `save!` instead of `save` if the return value is not checked.
    nil
  end
end

# CREATE in condition inside a block
items.each do |i|
  if User.create
          ^^^^^^ Rails/SaveBang: `create` returns a model which is always truthy.
    puts "yes"
  end
end

# CREATE in assignment inside a block (not followed by persisted?)
items.each do |i|
  x = User.create
           ^^^^^^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked. Or check `persisted?` on model returned from `create`.
  nil
end

# Persist call chained as receiver of non-persisted? method (return value not meaningfully checked)
def process
  object.save.to_s
         ^^^^ Rails/SaveBang: Use `save!` instead of `save` if the return value is not checked.
  object.update(name: 'Tom').inspect
         ^^^^^^ Rails/SaveBang: Use `update!` instead of `update` if the return value is not checked.
  nil
end

# Persist call as receiver of method chain inside argument context
# (outer expression is an argument, but the persist call itself is a receiver — not exempt)
log(object.save.to_s)
           ^^^^ Rails/SaveBang: Use `save!` instead of `save` if the return value is not checked.
result = object.update(name: 'Tom').inspect
                ^^^^^^ Rails/SaveBang: Use `update!` instead of `update` if the return value is not checked.

# Multi-statement method: last statement is NOT implicit return
# (RuboCop only exempts single-statement method/block bodies)
def multi_stmt_method
  setup_things
  object.save
         ^^^^ Rails/SaveBang: Use `save!` instead of `save` if the return value is not checked.
end

# Multi-statement block: last statement is NOT implicit return
items.each do |item|
  log(item)
  item.save
       ^^^^ Rails/SaveBang: Use `save!` instead of `save` if the return value is not checked.
end

# Multi-statement brace block
items.each { |item| log(item); item.update(name: 'Tom') }
                                    ^^^^^^ Rails/SaveBang: Use `update!` instead of `update` if the return value is not checked.

# Persist call in string interpolation (return value not checked)
def process
  "result: #{object.save}"
                    ^^^^ Rails/SaveBang: Use `save!` instead of `save` if the return value is not checked.
  nil
end

# Persist call in array literal in void context (NOT exempt)
def process_array
  [object.save]
          ^^^^ Rails/SaveBang: Use `save!` instead of `save` if the return value is not checked.
  nil
end

# Persist call in hash literal in void context (NOT exempt)
def process_hash
  {key: object.save}
               ^^^^ Rails/SaveBang: Use `save!` instead of `save` if the return value is not checked.
  nil
end

# Singleton method: implicit return does NOT apply (RuboCop only exempts def, not def self.x)
def self.create_default
  create(name: 'test')
  ^^^^^^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked.
end

# Block-wrapped create in argument context: create { block } as array element inside method arg
# In RuboCop, `create { }` becomes Block(Send, Args, Body) — argument? on the Send walks
# Send→Block→array, and Block.parent is array, not send_type?, so argument? returns false.
# RuboCop flags this.
def schedule_with_state
  Subscription.new([Item.create { setup }, Subscription.create { cleanup }])
                         ^^^^^^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked.
                                                        ^^^^^^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked.
end

# CREATE inside || or && (compound_boolean? in RuboCop) — always flagged as conditional
# regardless of enclosing context (assignment, argument, implicit return)
Tag.find_by_name("foo") || Tag.create(name: "foo")
                               ^^^^^^ Rails/SaveBang: `create` returns a model which is always truthy.
Setting.first || Setting.create(name: "bar")
                         ^^^^^^ Rails/SaveBang: `create` returns a model which is always truthy.
x = Foo.first || Foo.create(name: "baz")
                     ^^^^^^ Rails/SaveBang: `create` returns a model which is always truthy.
log(Thing.find || Thing.create(name: "qux"))
                        ^^^^^^ Rails/SaveBang: `create` returns a model which is always truthy.

# rescue modifier breaks implicit return and assignment chains
def teardown
  @post.destroy rescue nil
        ^^^^^^^ Rails/SaveBang: Use `destroy!` instead of `destroy` if the return value is not checked.
end
exception = (around.save rescue $!)
                    ^^^^ Rails/SaveBang: Use `save!` instead of `save` if the return value is not checked.

# yield arguments are NOT in argument context (RuboCop's argument? only checks send/csend parents)
items.each {|p| yield(Node.create(p)) }
                           ^^^^^^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked.

# Splat breaks argument context chain
execute *builder.create
                 ^^^^^^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked.

# yield with modify persist call (yield is NOT argument context per RuboCop)
def process_yield
  yield object.save
               ^^^^ Rails/SaveBang: Use `save!` instead of `save` if the return value is not checked.
  nil
end

# super with modify persist call (super is NOT argument context per RuboCop)
def process_super
  super(object.save)
               ^^^^ Rails/SaveBang: Use `save!` instead of `save` if the return value is not checked.
  nil
end

# yield/super even in implicit return position — yield/super break the chain
def process_yield_implicit
  yield object.save
               ^^^^ Rails/SaveBang: Use `save!` instead of `save` if the return value is not checked.
end
def process_super_implicit
  super(object.save)
               ^^^^ Rails/SaveBang: Use `save!` instead of `save` if the return value is not checked.
end

# Create in || inside setter assignment — compound_boolean should flag
self.parent_tag = Tag.find_by_name("x") || Tag.create(name: "x")
                                               ^^^^^^ Rails/SaveBang: `create` returns a model which is always truthy.

# Hash#update on hash literal — flagged as persist method
{ruby_method_type: :class}.update(kwargs)
                           ^^^^^^ Rails/SaveBang: Use `update!` instead of `update` if the return value is not checked.

# Create in || assigned to local (no persisted? check) — compound_boolean
x = AdminSetting.first || AdminSetting.create(last_updated_by: Admin.first)
                                       ^^^^^^ Rails/SaveBang: `create` returns a model which is always truthy.

# Create in || with memoization operator — compound_boolean
@current ||= current_user.presence || User.create(email: "x")
                                           ^^^^^^ Rails/SaveBang: `create` returns a model which is always truthy.

# Create chained: Student.create.lessons — create return value used as receiver chain
Student.create.lessons
        ^^^^^^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked.

# Create on LEFT side of `or`/`||` in block implicit return — NOT exempt
# (RuboCop's implicit_return? only exempts the right side via sibling_index math)
items.map { |v| Gem::Version.create(v) or raise }
                             ^^^^^^ Rails/SaveBang: `create` returns a model which is always truthy.

# Create in || inside instance variable assignment — compound_boolean takes priority
# (RuboCop's return_value_assigned? doesn't walk through or nodes)
@directory = connection.directories.get(key) || connection.directories.create(key: key)
                                                                       ^^^^^^ Rails/SaveBang: `create` returns a model which is always truthy.

# CREATE with csend persisted? — RuboCop's call_to_persisted? only matches send_type?, not csend
# So `s&.persisted?` does NOT count as a persisted? check
s = DomainSetup.create(domain: "x")
                ^^^^^^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked. Or check `persisted?` on model returned from `create`.
s if s&.persisted?

# CREATE as receiver of call_operator_write — receiver is void context
Student.create.lessons += [science]
        ^^^^^^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked.

# CREATE in || inside explicit return — compound_boolean takes priority
# RuboCop's explicit_return? uses assignable_node which doesn't walk through or nodes
def self.get_or_create(**opts)
  record = find_by(opts)
  return record || create(opts)
                   ^^^^^^ Rails/SaveBang: `create` returns a model which is always truthy.
end

# CREATE in local assignment at top-level (no persisted? check)
field = Chargify::SubscriptionMetafield.create name: 'internal info'
                                        ^^^^^^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked. Or check `persisted?` on model returned from `create`.

# Parenthesized persist call in argument — parens break argument? check
# RuboCop's argument? checks node.parent.send_type? and begin (parens) is not send_type?
@accounts << (@account.users.create name: "Daniel")
                             ^^^^^^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked.

# Parenthesized save in argument — same as above
assert ( cnpj_valido.save ), "CNPJ valido nao foi salvo."
                     ^^^^ Rails/SaveBang: Use `save!` instead of `save` if the return value is not checked.

# update/save inside array inside && — compound_boolean/Condition should not leak through arrays
# RuboCop's in_condition_or_compound_boolean? checks first non-begin ancestor, which is array (not &&)
@database_version ||= (version = raw_connection.oracle_server_version) &&
  [version.major, version.minor, version.update, version.patch]
                                         ^^^^^^ Rails/SaveBang: Use `update!` instead of `update` if the return value is not checked.

# CREATE reassigned to same variable after persisted? check — second create has no persisted? check
# RuboCop's VariableForce tracks each assignment separately; persisted? on an earlier assignment
# does NOT suppress a later re-assignment to the same variable.
def metafield_example
  mf = Chargify::CustomerMetafield.create name: 'test'
  mf.persisted?
  mf = Chargify::SubscriptionMetafield.create name: 'internal info'
                                       ^^^^^^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked. Or check `persisted?` on model returned from `create`.
  nil
end

# RDoc merge helpers: modify calls in multi-statement method bodies should still be flagged
def merge(mod, klass)
  klass.attributes.concat mod.attributes
  klass.method_list.concat mod.method_list
  klass.aliases.concat mod.aliases
  klass.external_aliases.concat mod.external_aliases
  klass.constants.concat mod.constants
  klass.includes.concat mod.includes

  klass.methods_hash.update mod.methods_hash
                     ^^^^^^ Rails/SaveBang: Use `update!` instead of `update` if the return value is not checked.
  klass.constants_hash.update mod.constants_hash
                       ^^^^^^ Rails/SaveBang: Use `update!` instead of `update` if the return value is not checked.

  klass.current_section = mod.current_section
  klass.in_files.concat mod.in_files
  klass.sections.concat mod.sections
  klass.unmatched_alias_lists = mod.unmatched_alias_lists
  klass.current_section = mod.current_section
  klass.visibility = mod.visibility

  klass.classes_hash.update mod.classes_hash
                     ^^^^^^ Rails/SaveBang: Use `update!` instead of `update` if the return value is not checked.
  klass.modules_hash.update mod.modules_hash
                     ^^^^^^ Rails/SaveBang: Use `update!` instead of `update` if the return value is not checked.
  klass.metadata.update mod.metadata
                 ^^^^^^ Rails/SaveBang: Use `update!` instead of `update` if the return value is not checked.

  klass.document_self = mod.received_nodoc ? nil : mod.document_self
  klass.document_children = mod.document_children
  klass.force_documentation = mod.force_documentation
  klass.done_documenting = mod.done_documenting
end

# Bare update in a multi-statement method body is void context
def each_port(&block)
  close_check
  update
  ^^^^^^ Rails/SaveBang: Use `update!` instead of `update` if the return value is not checked.
  @real.each_port(&block)
end

# Nested block/proc bodies do not exempt create calls
def self.logmethods(klass, useself = true)
  Puppet::Util::Log.eachlevel { |level|
    klass.send(:define_method, level, proc { |args|
      args = args.join(" ") if args.is_a?(Array)
      if useself
        Puppet::Util::Log.create(
                          ^^^^^^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked.
          :level => level,
          :source => self,
          :message => args
        )
      else
        Puppet::Util::Log.create(
                          ^^^^^^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked.
          :level => level,
          :message => args
        )
      end
    })
  }
end

# Create assigned to a local without persisted? should be flagged
def execute_windows(command, arguments, stdin, stdout, stderr)
  process_info = Process.create(
                         ^^^^^^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked. Or check `persisted?` on model returned from `create`.
    :command_line => command,
    :startup_info => {:stdin => stdin, :stdout => stdout, :stderr => stderr}
  )
  process_info.process_id
end

# Create in modifier-unless body is void context, not condition context
def store(time)
  self.create(time - 5) unless FileTest.exists?(self.path)
       ^^^^^^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked.
end

image_proxy = create(*args)
^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked. Or check `persisted?` on model returned from `create`.

rec = create(cbr)
^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked. Or check `persisted?` on model returned from `create`.

cat = create(classification)
^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked. Or check `persisted?` on model returned from `create`.

entry = create(classification.merge("parent_id" => parent.id))
^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked. Or check `persisted?` on model returned from `create`.

comp  = create(:resource => target, :compliant => compliant, :event_type => event, :timestamp => Time.now.utc)
^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked. Or check `persisted?` on model returned from `create`.

rec = create(event)
^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked. Or check `persisted?` on model returned from `create`.

event = create(e.merge(:event_type => event_type.to_s, :default => true, :enabled => true))
^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked. Or check `persisted?` on model returned from `create`.

feature = create(hash.except(:id))
^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked. Or check `persisted?` on model returned from `create`.

post = create category_name: 'Personal'
       ^^^^^^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked. Or check `persisted?` on model returned from `create`.
