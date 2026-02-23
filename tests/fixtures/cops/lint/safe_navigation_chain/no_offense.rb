x.foo.bar.baz
x&.foo&.bar&.baz
x.foo.bar&.baz
x&.foo == bar
x&.foo === bar
x&.foo || bar
x&.foo && bar
x&.foo | bar
x&.foo & bar
x&.foo.nil?
x&.foo.present?
x&.foo.blank?
x&.foo = bar
x&.foo += bar
+str&.to_i
-str&.to_i

# NilClass methods are safe to call after safe navigation
x&.foo.to_s
x&.foo.to_i
x&.foo.to_f
x&.foo.to_a
x&.foo.to_h
x&.foo.inspect
x&.foo.class
x&.foo.frozen?
x&.foo.is_a?(String)
x&.foo.respond_to?(:bar)
x&.foo.try(:bar)
x&.foo.presence

# Additional Object/Kernel methods safe on nil
x&.foo.tap { |x| puts x }
x&.foo.then { |x| x.to_s }
x&.foo.yield_self { |x| x }
x&.foo.itself
x&.foo.dup
x&.foo.clone
x&.foo.freeze
x&.foo.hash
x&.foo.object_id
x&.foo.send(:bar)
x&.foo.public_send(:bar)
x&.foo.instance_eval { @x }
x&.foo.eql?(other)
x&.foo.kind_of?(String)
x&.foo.instance_of?(String)
x&.foo.singleton_class
x&.foo.method(:to_s)
x&.foo.to_enum
x&.foo.enum_for(:each)
x&.foo.instance_variables
x&.foo.methods
x&.foo.respond_to_missing?(:bar, false)
x&.foo.pp
x&.foo.pretty_inspect
