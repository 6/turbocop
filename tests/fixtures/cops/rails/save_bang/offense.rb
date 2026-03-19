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
