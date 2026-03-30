class A
  @@test = 10
  ^^^^^^ Style/ClassVars: Replace class var @@test with a class instance var.
end

class B
  @@count = 0
  ^^^^^^^ Style/ClassVars: Replace class var @@count with a class instance var.
end

class C
  @@name = "test"
  ^^^^^^ Style/ClassVars: Replace class var @@name with a class instance var.
end

@@username, @@password = @@ccm_cluster.enable_authentication
^^^^^^^^^^ Style/ClassVars: Replace class var @@username with a class instance var.
            ^^^^^^^^^^ Style/ClassVars: Replace class var @@password with a class instance var.

@@server_cert, @@client_cert, @@private_key, @@passphrase = @@ccm_cluster.enable_ssl_client_auth
^^^^^^^^^^^^^ Style/ClassVars: Replace class var @@server_cert with a class instance var.
               ^^^^^^^^^^^^^ Style/ClassVars: Replace class var @@client_cert with a class instance var.
                              ^^^^^^^^^^^^^ Style/ClassVars: Replace class var @@private_key with a class instance var.
                                             ^^^^^^^^^^^^ Style/ClassVars: Replace class var @@passphrase with a class instance var.

@@choices, @@rest = Parser.parse(@@options, @@args)
^^^^^^^^^ Style/ClassVars: Replace class var @@choices with a class instance var.
           ^^^^^^ Style/ClassVars: Replace class var @@rest with a class instance var.

@@warden_config, @@warden_config_blocks = c, b
^^^^^^^^^^^^^^^ Style/ClassVars: Replace class var @@warden_config with a class instance var.
                 ^^^^^^^^^^^^^^^^^^^^^^ Style/ClassVars: Replace class var @@warden_config_blocks with a class instance var.

_port, @@remote_ip = Socket.unpack_sockaddr_in(get_peername)
       ^^^^^^^^^^^ Style/ClassVars: Replace class var @@remote_ip with a class instance var.

@@shard1, @@shard2 = TestHelper.recreate_persistent_test_shards
^^^^^^^^ Style/ClassVars: Replace class var @@shard1 with a class instance var.
          ^^^^^^^^ Style/ClassVars: Replace class var @@shard2 with a class instance var.

@@extended_fields, @@topic_types = [], []
^^^^^^^^^^^^^^^^^ Style/ClassVars: Replace class var @@extended_fields with a class instance var.
                   ^^^^^^^^^^^^^ Style/ClassVars: Replace class var @@topic_types with a class instance var.

@@prev, @@i = nil, 0
^^^^^^ Style/ClassVars: Replace class var @@prev with a class instance var.
        ^^^ Style/ClassVars: Replace class var @@i with a class instance var.

(@@a, @@b), @@c = foo
 ^^^ Style/ClassVars: Replace class var @@a with a class instance var.
      ^^^ Style/ClassVars: Replace class var @@b with a class instance var.
            ^^^ Style/ClassVars: Replace class var @@c with a class instance var.

class RescueCapture
  def capture(msg)
    raise msg
  rescue => @@captured_error
            ^^^^^^^^^^^^^^^^ Style/ClassVars: Replace class var @@captured_error with a class instance var.
    :caught
  end
end

class ForLoopOne
  m = [1, 2, 3]
  for @@var in m
      ^^^^^ Style/ClassVars: Replace class var @@var with a class instance var.
    m
  end
end

class RescueFoo
  def foo
    begin
      raise "foo"
    rescue => @@e
              ^^^ Style/ClassVars: Replace class var @@e with a class instance var.
    end
    @@e
  end
end

class ForLoopTwo
  m = [1, 2, 3]
  for @@var in m
      ^^^^^ Style/ClassVars: Replace class var @@var with a class instance var.
    m
  end
end
