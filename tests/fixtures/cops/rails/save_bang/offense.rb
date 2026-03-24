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
