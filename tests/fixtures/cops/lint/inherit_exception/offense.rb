class MyError < Exception; end
                ^^^^^^^^^ Lint/InheritException: Inherit from `StandardError` instead of `Exception`.

C = Class.new(Exception)
              ^^^^^^^^^ Lint/InheritException: Inherit from `StandardError` instead of `Exception`.

class AnotherError < ::Exception; end
                     ^^^^^^^^^^^ Lint/InheritException: Inherit from `StandardError` instead of `Exception`.
