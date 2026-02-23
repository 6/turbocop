foo && foo.bar
^^^^^^^^^^^^^^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

foo && foo.bar(param1, param2)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

foo && foo.bar.baz
^^^^^^^^^^^^^^^^^^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

foo.nil? ? nil : foo.bar
^^^^^^^^^^^^^^^^^^^^^^^^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

foo ? foo.bar : nil
^^^^^^^^^^^^^^^^^^^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

!foo.nil? ? foo.bar : nil
^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

callback.call unless callback.nil?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

handler.process unless handler.nil?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

obj.bar if obj
^^^^^^^^^^^^^^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.
