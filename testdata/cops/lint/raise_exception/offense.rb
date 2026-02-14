raise Exception, "message"
^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/RaiseException: Use a subclass of `Exception` instead of raising `Exception` directly.
raise Exception.new("message")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/RaiseException: Use a subclass of `Exception` instead of raising `Exception` directly.
fail Exception
^^^^^^^^^^^^^^ Lint/RaiseException: Use a subclass of `Exception` instead of raising `Exception` directly.
