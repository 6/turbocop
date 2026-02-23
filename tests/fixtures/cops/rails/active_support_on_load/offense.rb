ActiveRecord::Base.include(MyClass)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ActiveSupportOnLoad: Use `ActiveSupport.on_load(:active_record) { include ... }` instead of `ActiveRecord::Base.include(...)`.

ActiveRecord::Base.prepend(MyClass)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ActiveSupportOnLoad: Use `ActiveSupport.on_load(:active_record) { prepend ... }` instead of `ActiveRecord::Base.prepend(...)`.

ActiveRecord::Base.extend(MyClass)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ActiveSupportOnLoad: Use `ActiveSupport.on_load(:active_record) { extend ... }` instead of `ActiveRecord::Base.extend(...)`.

ActionController::Base.include(MyModule)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ActiveSupportOnLoad: Use `ActiveSupport.on_load(:action_controller) { include ... }` instead of `ActionController::Base.include(...)`.
