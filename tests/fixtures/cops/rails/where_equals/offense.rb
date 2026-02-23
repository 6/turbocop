User.where('name = ?', 'Gabe')
     ^^^^^ Rails/WhereEquals: Use `where(attribute: value)` instead of manually constructing SQL.
User.where('name IS NULL')
     ^^^^^ Rails/WhereEquals: Use `where(attribute: value)` instead of manually constructing SQL.
User.where('name IN (?)', ['john', 'jane'])
     ^^^^^ Rails/WhereEquals: Use `where(attribute: value)` instead of manually constructing SQL.
