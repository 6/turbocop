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

# CREATE methods in assignments should be flagged (return value not checked with persisted?)
def create_examples
  x = object.create
             ^^^^^^ Rails/SaveBang: Use `create!` instead of `create` if the return value is not checked. Or check `persisted?` on model returned from `create`.
  y = object.find_or_create_by(name: 'Tom')
             ^^^^^^^^^^^^^^^^^^ Rails/SaveBang: Use `find_or_create_by!` instead of `find_or_create_by` if the return value is not checked. Or check `persisted?` on model returned from `find_or_create_by`.
  @record = object.first_or_create
                   ^^^^^^^^^^^^^^^ Rails/SaveBang: Use `first_or_create!` instead of `first_or_create` if the return value is not checked. Or check `persisted?` on model returned from `first_or_create`.
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
