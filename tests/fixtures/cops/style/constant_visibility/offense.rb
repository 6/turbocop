class Foo
  BAR = 42
  ^^^^^^^^ Style/ConstantVisibility: Explicitly make `BAR` public or private using either `#public_constant` or `#private_constant`.
end

module Baz
  QUX = 'hello'
  ^^^^^^^^^^^^^ Style/ConstantVisibility: Explicitly make `QUX` public or private using either `#public_constant` or `#private_constant`.
end

class Quux
  include Bar
  BAZ = 42
  ^^^^^^^^ Style/ConstantVisibility: Explicitly make `BAZ` public or private using either `#public_constant` or `#private_constant`.
  private_constant :FOO
end

module Test
  IndexMapping::Interpolation = ::Google::Protobuf::DescriptorPool.generated_pool.lookup("test").enummodule
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConstantVisibility: Explicitly make `Interpolation` public or private using either `#public_constant` or `#private_constant`.
end

class InstallGenerator
  ::InvalidChannel = InvalidChannel
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConstantVisibility: Explicitly make `InvalidChannel` public or private using either `#public_constant` or `#private_constant`.
  ::ConflictingOptions = ConflictingOptions
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConstantVisibility: Explicitly make `ConflictingOptions` public or private using either `#public_constant` or `#private_constant`.
end

module Skyline
  class Engine
    Skyline::Engine::SESSION_OPTIONS = {}
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConstantVisibility: Explicitly make `SESSION_OPTIONS` public or private using either `#public_constant` or `#private_constant`.
  end
end

module Proto
  Trace::CachePolicy = lookup("Trace.CachePolicy").msgclass
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConstantVisibility: Explicitly make `CachePolicy` public or private using either `#public_constant` or `#private_constant`.
  Trace::CachePolicy::Scope = lookup("Trace.CachePolicy.Scope").enummodule
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConstantVisibility: Explicitly make `Scope` public or private using either `#public_constant` or `#private_constant`.
end

module Backports
  class FilteredQueue
    CONSUME_ON_ESCAPE = true
    ^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConstantVisibility: Explicitly make `CONSUME_ON_ESCAPE` public or private using either `#public_constant` or `#private_constant`.
  end

  class Ractor
    class BaseQueue < FilteredQueue
      ClosedQueueError = Ractor::ClosedError
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConstantVisibility: Explicitly make `ClosedQueueError` public or private using either `#public_constant` or `#private_constant`.
    end

    class IncomingQueue < BaseQueue
      TYPE = :incoming
      ^^^^^^^^^^^^^^^^ Style/ConstantVisibility: Explicitly make `TYPE` public or private using either `#public_constant` or `#private_constant`.
    end

    class OutgoingQueue < BaseQueue
      TYPE = :outgoing
      ^^^^^^^^^^^^^^^^ Style/ConstantVisibility: Explicitly make `TYPE` public or private using either `#public_constant` or `#private_constant`.
      WrappedException = ::Struct.new(:exception, :ractor)
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConstantVisibility: Explicitly make `WrappedException` public or private using either `#public_constant` or `#private_constant`.
    end
  end
end

class Net::IMAP::FakeServer
  class Configuration
    CA_FILE     = File.expand_path("../../fixtures/cacert.pem", __dir__)
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConstantVisibility: Explicitly make `CA_FILE` public or private using either `#public_constant` or `#private_constant`.
    SERVER_KEY  = File.expand_path("../../fixtures/server.key", __dir__)
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConstantVisibility: Explicitly make `SERVER_KEY` public or private using either `#public_constant` or `#private_constant`.
    SERVER_CERT = File.expand_path("../../fixtures/server.crt", __dir__)
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ConstantVisibility: Explicitly make `SERVER_CERT` public or private using either `#public_constant` or `#private_constant`.
    DEFAULTS = {
    ^^^^^^^^^^^^ Style/ConstantVisibility: Explicitly make `DEFAULTS` public or private using either `#public_constant` or `#private_constant`.
      tls: { ca_file: CA_FILE, key: SERVER_KEY, cert: SERVER_CERT }.freeze,
    }
  end
end
