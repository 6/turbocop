user.update(name: "new")
user.save
user.save!
User.create(name: "new")
User.find_or_create_by(name: "test")
# String#insert is not an AR method — good_insert? check
path.insert(index + 1, '_')
string.insert(0, 'b')
string&.insert(0, 'b')
# Array#insert is not an AR method
array.insert(1, :a, :b)
array&.insert(1, :a, :b)
# insert with non-AR keyword args — good_insert? check
insert(attributes, something_else: true)
# insert with mixed keyword args including :returning but also other keys
insert(attributes, returning: false, something_else: true)
# insert with block argument — not AR insert (block_pass counts as an arg in RuboCop AST)
operand.insert(*args, &bl)
insert(node, &block)
dataset.insert_conflict(opts).insert(*values, &)
# FileUtils.touch is not a model method
FileUtils.touch('file')
::FileUtils.touch('file')
# touch with boolean arg is not a model skip
record.touch(true)
belongs_to(:user).touch(false)
# METHODS_WITH_ARGUMENTS: no args means not a model validation skip
User.toggle!
User.increment!
User.decrement!
User.insert
User.insert!
User.insert_all
User.insert_all!
User.update_all
User.update_attribute
User.update_column
User.update_columns
User.update_counters
User.upsert
User.upsert_all
User.increment_counter
User.decrement_counter
