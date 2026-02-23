IO.select([io], [], [], timeout)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/IncompatibleIoSelectWithFiberScheduler: Use `io.wait_readable(timeout)` instead of `IO.select([io], [], [], timeout)`.
IO.select([], [io], [], timeout)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/IncompatibleIoSelectWithFiberScheduler: Use `io.wait_writable(timeout)` instead of `IO.select([], [io], [], timeout)`.
IO.select([io], [])
^^^^^^^^^^^^^^^^^^^ Lint/IncompatibleIoSelectWithFiberScheduler: Use `io.wait_readable` instead of `IO.select([io], [])`.
