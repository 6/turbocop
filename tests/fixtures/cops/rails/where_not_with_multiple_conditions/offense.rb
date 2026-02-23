User.where.not(trashed: true, role: 'admin')
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/WhereNotWithMultipleConditions: Use a SQL statement instead of `where.not` with multiple conditions.
User.where.not(trashed: true, role: ['moderator', 'admin'])
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/WhereNotWithMultipleConditions: Use a SQL statement instead of `where.not` with multiple conditions.
User.where.not(active: false, banned: true)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/WhereNotWithMultipleConditions: Use a SQL statement instead of `where.not` with multiple conditions.
