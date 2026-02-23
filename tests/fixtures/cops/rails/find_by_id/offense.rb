User.find_by_id!(id)
     ^^^^^^^^^^^^ Rails/FindById: Use `find` instead of `find_by_id!`.
User.find_by!(id: id)
     ^^^^^^^^ Rails/FindById: Use `find` instead of `find_by!`.
User.where(id: id).take!
     ^^^^^ Rails/FindById: Use `find` instead of `where(id: ...).take!`.
