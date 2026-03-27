fail RuntimeError, "message"
^^^^ Style/SignalException: Use `raise` instead of `fail` to rethrow exceptions.

fail "something went wrong"
^^^^ Style/SignalException: Use `raise` instead of `fail` to rethrow exceptions.

fail ArgumentError, "bad argument"
^^^^ Style/SignalException: Use `raise` instead of `fail` to rethrow exceptions.

fail "Could not find running ssh agent - Is config.ssh.forward_agent enabled in Vagrantfile?" unless ENV['SSH_AUTH_SOCK']
^^^^ Style/SignalException: Use `raise` instead of `fail` to rethrow exceptions.
