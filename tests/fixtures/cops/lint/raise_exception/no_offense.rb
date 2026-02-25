raise StandardError, "message"
raise RuntimeError.new("message")
raise "message"
fail ArgumentError, "bad"
raise TypeError
raise ::StandardError, "qualified"
raise ::Foreman::Exception.new("msg")
raise Foreman::Exception.new("msg")
raise MyApp::Exception
raise Foo::Bar::Exception, "namespaced"
