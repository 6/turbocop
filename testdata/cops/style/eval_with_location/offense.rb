eval "do_something"
^^^^^^^^^^^^^^^^^^^ Style/EvalWithLocation: Pass a binding, `__FILE__`, and `__LINE__` to `eval`.
eval "do_something", binding
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/EvalWithLocation: Pass a binding, `__FILE__`, and `__LINE__` to `eval`.
eval "do_something", binding, __FILE__
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/EvalWithLocation: Pass a binding, `__FILE__`, and `__LINE__` to `eval`.
C.class_eval "do_something"
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/EvalWithLocation: Pass `__FILE__` and `__LINE__` to `class_eval`.
M.module_eval "do_something"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/EvalWithLocation: Pass `__FILE__` and `__LINE__` to `module_eval`.
foo.instance_eval "do_something"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/EvalWithLocation: Pass `__FILE__` and `__LINE__` to `instance_eval`.
