eval "do_something", binding, __FILE__, __LINE__
C.class_eval "do_something", __FILE__, __LINE__ + 1
M.module_eval "do_something", __FILE__, __LINE__ + 1
foo.instance_eval "do_something", __FILE__, __LINE__
foo.eval "CODE"
code = something
eval code
