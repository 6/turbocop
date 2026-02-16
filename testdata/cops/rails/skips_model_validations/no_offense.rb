user.update(name: "new")
user.save
user.save!
User.create(name: "new")
User.find_or_create_by(name: "test")
# String#insert is not an AR method — good_insert? check
path.insert(index + 1, '_')
# FileUtils.touch is not a model method
FileUtils.touch('file')
# touch with boolean arg is not a model skip
record.touch(true)
