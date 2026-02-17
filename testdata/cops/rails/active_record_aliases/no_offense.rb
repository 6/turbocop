book.update(author: 'Alice')
book.update!(author: 'Alice')
user.update(name: 'Bob', age: 30)
record.update!(status: :active)
# Non-receiver calls are not flagged
update_attributes(name: 'Bob')
