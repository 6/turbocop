book.update_attributes(author: 'Alice')
     ^^^^^^^^^^^^^^^^^ Rails/ActiveRecordAliases: Use `update` instead of `update_attributes`.
book.update_attributes!(author: 'Alice')
     ^^^^^^^^^^^^^^^^^^ Rails/ActiveRecordAliases: Use `update!` instead of `update_attributes!`.
user.update_attributes(name: 'Bob', age: 30)
     ^^^^^^^^^^^^^^^^^ Rails/ActiveRecordAliases: Use `update` instead of `update_attributes`.
record.update_attributes!(status: :active)
       ^^^^^^^^^^^^^^^^^^ Rails/ActiveRecordAliases: Use `update!` instead of `update_attributes!`.
update_attributes(name: 'Bob')
^^^^^^^^^^^^^^^^^ Rails/ActiveRecordAliases: Use `update` instead of `update_attributes`.
update_attributes!(status: :active)
^^^^^^^^^^^^^^^^^^ Rails/ActiveRecordAliases: Use `update!` instead of `update_attributes!`.
update_attributes(params, *options)
^^^^^^^^^^^^^^^^^ Rails/ActiveRecordAliases: Use `update` instead of `update_attributes`.
