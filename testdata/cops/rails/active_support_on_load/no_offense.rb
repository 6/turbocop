ActiveSupport.on_load(:active_record) { include MyClass }
ActiveSupport.on_load(:action_controller) do
  include MyModule
end
foo.extend(MyClass)
MyClass1.prepend(MyClass)
ActiveRecord::Base.include
name.include?('bob')
