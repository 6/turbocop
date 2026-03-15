ActiveRecord::Base.include(MyClass)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ActiveSupportOnLoad: Use `ActiveSupport.on_load(:active_record) { include ... }` instead of `ActiveRecord::Base.include(...)`.

ActiveRecord::Base.prepend(MyClass)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ActiveSupportOnLoad: Use `ActiveSupport.on_load(:active_record) { prepend ... }` instead of `ActiveRecord::Base.prepend(...)`.

ActiveRecord::Base.extend(MyClass)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ActiveSupportOnLoad: Use `ActiveSupport.on_load(:active_record) { extend ... }` instead of `ActiveRecord::Base.extend(...)`.

ActionController::Base.include(MyModule)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ActiveSupportOnLoad: Use `ActiveSupport.on_load(:action_controller) { include ... }` instead of `ActionController::Base.include(...)`.

ActiveSupport::TestCase.include(MyModule)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ActiveSupportOnLoad: Use `ActiveSupport.on_load(:active_support_test_case) { include ... }` instead of `ActiveSupport::TestCase.include(...)`.

ActionDispatch::IntegrationTest.include(MyModule)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ActiveSupportOnLoad: Use `ActiveSupport.on_load(:action_dispatch_integration_test) { include ... }` instead of `ActionDispatch::IntegrationTest.include(...)`.

ActiveStorage::Blob.prepend(MyModule)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ActiveSupportOnLoad: Use `ActiveSupport.on_load(:active_storage_blob) { prepend ... }` instead of `ActiveStorage::Blob.prepend(...)`.

ActionMailbox::Base.extend(MyModule)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ActiveSupportOnLoad: Use `ActiveSupport.on_load(:action_mailbox) { extend ... }` instead of `ActionMailbox::Base.extend(...)`.

ActiveModel::Model.include(MyModule)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ActiveSupportOnLoad: Use `ActiveSupport.on_load(:active_model) { include ... }` instead of `ActiveModel::Model.include(...)`.
