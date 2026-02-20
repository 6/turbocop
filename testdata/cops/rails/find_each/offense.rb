User.all.each { |u| u.save }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.
User.where(active: true).each { |u| u.save }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.
User.includes(:posts).each { |u| u.save }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.
User.joins(:posts).each { |u| u.save }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.