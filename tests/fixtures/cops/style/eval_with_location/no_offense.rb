eval "do_something", binding, __FILE__, __LINE__
C.class_eval "do_something", __FILE__, __LINE__
M.module_eval "do_something", __FILE__, __LINE__
foo.instance_eval "do_something", __FILE__, __LINE__
foo.eval "CODE"
eval `git show HEAD:foo.rb`
code = something
eval code
eval()
C.class_eval <<-RUBY, __FILE__, __LINE__ + 1
  code
RUBY
module_eval(<<~CODE, __FILE__, lineno)
  do_something
CODE
def self.included(base)
  base.class_eval do
    include OtherModule
  end
end
