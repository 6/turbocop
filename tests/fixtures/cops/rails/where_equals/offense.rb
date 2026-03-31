User.where('name = ?', 'Gabe')
     ^^^^^ Rails/WhereEquals: Use `where(attribute: value)` instead of manually constructing SQL.
User.where('name IS NULL')
     ^^^^^ Rails/WhereEquals: Use `where(attribute: value)` instead of manually constructing SQL.
User.where('name IN (?)', ['john', 'jane'])
     ^^^^^ Rails/WhereEquals: Use `where(attribute: value)` instead of manually constructing SQL.
User.where(['name = ?', 'Gabe'])
     ^^^^^ Rails/WhereEquals: Use `where(attribute: value)` instead of manually constructing SQL.
User.where(['name IS NULL'])
     ^^^^^ Rails/WhereEquals: Use `where(attribute: value)` instead of manually constructing SQL.
User.where(["name IN (?)", ['john', 'jane']])
     ^^^^^ Rails/WhereEquals: Use `where(attribute: value)` instead of manually constructing SQL.
User.where(['name = :name', { name: 'Gabe' }])
     ^^^^^ Rails/WhereEquals: Use `where(attribute: value)` instead of manually constructing SQL.
User.where('name = :name', name: 'Gabe')
     ^^^^^ Rails/WhereEquals: Use `where(attribute: value)` instead of manually constructing SQL.
Course.where(['enrollments.student_id = ?', student.id])
       ^^^^^ Rails/WhereEquals: Use `where(attribute: value)` instead of manually constructing SQL.
scope :active, -> { where('active = ?', true) }
                    ^^^^^ Rails/WhereEquals: Use `where(attribute: value)` instead of manually constructing SQL.
