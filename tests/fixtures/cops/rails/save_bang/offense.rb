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
