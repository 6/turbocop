expect(foo).to receive(:bar).and_return('hello world')
^^^^^^^^^^^ RSpec/StubbedMock: Prefer `allow` over `expect` when configuring a response.
expect(foo).to receive(:bar) { 'hello world' }
^^^^^^^^^^^ RSpec/StubbedMock: Prefer `allow` over `expect` when configuring a response.
expect(foo).to receive_messages(foo: 42, bar: 777)
^^^^^^^^^^^ RSpec/StubbedMock: Prefer `allow` over `expect` when configuring a response.
