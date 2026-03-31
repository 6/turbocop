a, b, c = 1, 2, 3
^^^^^^^^^^^^^^^^^^ Style/ParallelAssignment: Do not use parallel assignment.

x, y = "hello", "world"
^^^^^^^^^^^^^^^^^^^^^^^^ Style/ParallelAssignment: Do not use parallel assignment.

a, b = foo(), bar()
^^^^^^^^^^^^^^^^^^^ Style/ParallelAssignment: Do not use parallel assignment.

@name, @config, @bulk, = name, config, bulk
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ParallelAssignment: Do not use parallel assignment.

@value, options = options, {} unless Hash === options
^ Style/ParallelAssignment: Do not use parallel assignment.

suppressed_was, @suppressed = @suppressed, true
^ Style/ParallelAssignment: Do not use parallel assignment.

suppressed_was, @suppressed = @suppressed, false
^ Style/ParallelAssignment: Do not use parallel assignment.

old, $VERBOSE = $VERBOSE, nil
^ Style/ParallelAssignment: Do not use parallel assignment.

state, opts = opts, nil
^ Style/ParallelAssignment: Do not use parallel assignment.

state, opts = opts, nil
^ Style/ParallelAssignment: Do not use parallel assignment.

state, opts = opts, nil
^ Style/ParallelAssignment: Do not use parallel assignment.

server_was, @_current_server = @_current_server, nil
^ Style/ParallelAssignment: Do not use parallel assignment.
