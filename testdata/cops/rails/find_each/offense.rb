User.all.each { |u| u.save }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.
User.where(active: true).each { |u| u.save }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.
User.order(:name).each { |u| u.save }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.
User.select(:name).each { |u| u.save }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.