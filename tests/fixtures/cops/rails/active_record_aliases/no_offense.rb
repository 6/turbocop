book.update(author: 'Alice')
book.update!(author: 'Alice')
user.update(name: 'Bob', age: 30)
record.update!(status: :active)
# update_attributes without arguments is not flagged (different method)
update_attributes
