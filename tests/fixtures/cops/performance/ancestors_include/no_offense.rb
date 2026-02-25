Foo.is_a?(Bar)
obj.ancestors
Class <= Kernel
foo.include?(bar)
ancestors.select { |a| a == Foo }
# Non-constant receivers should not be flagged
object_one.ancestors.include?(object_two)
self.class.ancestors.include?(Fluent::Compat::CallSuperMixin)
@formatter.class.ancestors.include?(Fluent::Compat::HandleTagAndTimeMixin)
klass.ancestors.include?(SomeMixin)
node.ancestors.include?(target)
