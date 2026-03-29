def m
  items.something { |i| yield i }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ExplicitBlockArgument: Consider using explicit block argument in the surrounding method's signature over `yield`.
end

def n
  items.each { |x| yield x }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ExplicitBlockArgument: Consider using explicit block argument in the surrounding method's signature over `yield`.
end

def o
  foo.bar { |a, b| yield a, b }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ExplicitBlockArgument: Consider using explicit block argument in the surrounding method's signature over `yield`.
end

def p
  3.times { yield }
  ^^^^^^^^^^^^^^^^^^ Style/ExplicitBlockArgument: Consider using explicit block argument in the surrounding method's signature over `yield`.
end

def q
  super { yield }
  ^^^^^^^^^^^^^^^^ Style/ExplicitBlockArgument: Consider using explicit block argument in the surrounding method's signature over `yield`.
end

def r
  RestClient::Request.new(request_options).execute { |*params| yield params }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ExplicitBlockArgument: Consider using explicit block argument in the surrounding method's signature over `yield`.
end

def s(name, attributes)
  metric_instrumentation.time(name, attributes, -> { yield })
                                                ^^^^^^^^^^^^ Style/ExplicitBlockArgument: Consider using explicit block argument in the surrounding method's signature over `yield`.
end

def t(name, attributes)
  metric_instrumentation.histogram(name, attributes, -> { yield })
                                                     ^^^^^^^^^^^^ Style/ExplicitBlockArgument: Consider using explicit block argument in the surrounding method's signature over `yield`.
end

def u(name, attributes)
  metric_instrumentation.increment_counter(name, attributes, -> { yield })
                                                             ^^^^^^^^^^^^ Style/ExplicitBlockArgument: Consider using explicit block argument in the surrounding method's signature over `yield`.
end

def v(name, attributes)
  metric_instrumentation.decrement_counter(name, attributes, -> { yield })
                                                             ^^^^^^^^^^^^ Style/ExplicitBlockArgument: Consider using explicit block argument in the surrounding method's signature over `yield`.
end

def w(name, attributes)
  metric_instrumentation.gauge(name, attributes, -> { yield })
                                                 ^^^^^^^^^^^^ Style/ExplicitBlockArgument: Consider using explicit block argument in the surrounding method's signature over `yield`.
end

def x
  ->{ yield }.call
  ^^^^^^^^^^^ Style/ExplicitBlockArgument: Consider using explicit block argument in the surrounding method's signature over `yield`.
end

def y(hooks)
  chain = hooks.reverse.reduce(-> { yield }) do |inner, hook|
                               ^^^^^^^^^^^^ Style/ExplicitBlockArgument: Consider using explicit block argument in the surrounding method's signature over `yield`.
    -> { instance_exec(inner, &hook) }
  end
end
