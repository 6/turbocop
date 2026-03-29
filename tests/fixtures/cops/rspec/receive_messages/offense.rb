before do
  allow(Service).to receive(:foo).and_return(baz)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
  allow(Service).to receive(:bar).and_return(true)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
  allow(Service).to receive(:baz).and_return("x")
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
end

# Stubs in a method body (def) should also be detected
def setup_stubs
  allow(Service).to receive(:name).and_return("test")
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
  allow(Service).to receive(:status).and_return(:active)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
end

# Duplicate receive args: unique items should still be flagged
before do
  allow(Service).to receive(:foo).and_return(bar)
  allow(Service).to receive(:bar).and_return(qux)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
  allow(Service).to receive(:foo).and_return(qux)
  allow(Service).to receive(:baz).and_return(qux)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
end

# Stubs with other statements between them
before do
  allow(Service).to receive(:alpha).and_return(1)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
  call_something
  allow(Service).to receive(:beta).and_return(2)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
end

# Stubs inside begin...ensure should still be detected.
begin
  allow(file_upload).to receive(:extract_zip_to_tmp_dir).and_return({
  ^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
    'cfg' => [cfg_file.path, server_cfg_path, reservation_cfg_path, cp_cfg_path],
    'maps' => ['/tmp/cp_badlands.bsp']
  })
  expect(file_upload).to be_valid

  file_upload.save!
  allow(file_upload).to receive(:upload_files_to_servers).and_return([])
  ^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
ensure
  cleanup
end

# Multiline and_return arguments inside begin...ensure should also be detected.
begin
  allow(config).to receive(:ca_file).and_return(
  ^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
    File.expand_path(File.join(File.dirname(__FILE__), "..", "..", "ssl", "geotrust_global.crt")),
  )
  allow(config).to receive(:ssl?).and_return(true)
  ^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
  allow(config).to receive(:port).and_return(SSL_TEST_PORT)
  ^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
ensure
  cleanup
end

# Stubs in an example body should be detected.
RSpec.describe do
  it do
    service = described_class.new(channel: channel, content: content)
    allow(service).to receive(:create_contact_inbox).and_return(contact_inbox)
    ^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
    allow(service).to receive(:fetch_attachment).and_return(tempfile)
    ^ RSpec/ReceiveMessages: Use `receive_messages` instead of multiple stubs.
  end
end
