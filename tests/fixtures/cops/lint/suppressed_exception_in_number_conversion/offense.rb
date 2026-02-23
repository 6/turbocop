Integer(arg) rescue nil
^^^^^^^^^^^^^^^^^^^^^^^ Lint/SuppressedExceptionInNumberConversion: Use `Integer(arg, exception: false)` instead.
Float(arg) rescue nil
^^^^^^^^^^^^^^^^^^^^^ Lint/SuppressedExceptionInNumberConversion: Use `Float(arg, exception: false)` instead.
Complex(arg) rescue nil
^^^^^^^^^^^^^^^^^^^^^^^ Lint/SuppressedExceptionInNumberConversion: Use `Complex(arg, exception: false)` instead.
