hash.dig('foo', 'bar')
hash.fetch('foo', nil)
hash.fetch('foo').fetch('bar')
hash.fetch('foo') { :default }.fetch('bar') { :default }
hash.fetch('foo', bar).fetch('baz', quux)
hash.fetch('foo', {}).fetch('bar', {})
