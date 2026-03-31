io.wait_readable(timeout)
io.wait_writable(timeout)
IO.select([io1, io2])
IO.select([io], [], [err])
IO.select([a], [b])
expect(LightIO::Library::IO.select([r], [], [], 0.0001)).to be_nil
read_fds, write_fds = LightIO::Library::IO.select([r1], nil, nil, 0)
expect {
  LightIO::Library::IO.select([1], nil)
}.to raise_error(TypeError)
expect {
  LightIO::Library::IO.select([B_TO_IO.new], nil)
}.to raise_error(TypeError)
r_fds, w_fds = LightIO::Library::IO.select(nil, [w])
LightIO::IO.select([serv])
