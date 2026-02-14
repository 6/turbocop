User.find_by_name("foo")
^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DynamicFindBy: Use `find_by(name: ...)` instead of `find_by_name`.
User.find_by_email("test@test.com")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DynamicFindBy: Use `find_by(email: ...)` instead of `find_by_email`.
Post.find_by_title("hello")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DynamicFindBy: Use `find_by(title: ...)` instead of `find_by_title`.
