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
class_eval <<-RUBY, __FILE__, __LINE__
^ Style/EvalWithLocation: Incorrect line number for `class_eval`; use `__LINE__ + 1` instead of `__LINE__`.
  code
RUBY
eval(%{ raise SyntaxError }, nil, "my_file.rb", 123)
^ Style/EvalWithLocation: Incorrect file for `eval`; use `__FILE__` instead of `"my_file.rb"`.
^ Style/EvalWithLocation: Incorrect line number for `eval`; use `__LINE__` instead of `123`.
generated_attribute_methods.module_eval <<-RUBY, __FILE__, __LINE__
^ Style/EvalWithLocation: Incorrect line number for `module_eval`; use `__LINE__ + 1` instead of `__LINE__`.
  code
RUBY
module_eval <<-RUBY, __FILE__, __LINE__
^ Style/EvalWithLocation: Incorrect line number for `module_eval`; use `__LINE__ + 1` instead of `__LINE__`.
  code
RUBY
C.class_eval "do_something", __FILE__, __LINE__ + 1
^ Style/EvalWithLocation: Incorrect line number for `class_eval`; use `__LINE__` instead of `__LINE__ + 1`.
M.module_eval "do_something", __FILE__, __LINE__ + 1
^ Style/EvalWithLocation: Incorrect line number for `module_eval`; use `__LINE__` instead of `__LINE__ + 1`.

eval "test passes" do
^ Style/EvalWithLocation: Pass a binding, `__FILE__`, and `__LINE__` to `eval`.
  true
end

mod.module_eval(<<~RUBY, loc[:file], loc[:line])
^ Style/EvalWithLocation: Incorrect file for `module_eval`; use `__FILE__` instead of `loc[:file]`.
  def example
  end
RUBY
