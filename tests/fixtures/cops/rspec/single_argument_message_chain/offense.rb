before do
  allow(foo).to receive_message_chain(:one) { :two }
                ^^^^^^^^^^^^^^^^^^^^^ RSpec/SingleArgumentMessageChain: Use `receive` instead of calling `receive_message_chain` with a single argument.
end

before do
  allow(foo).to receive_message_chain("one") { :two }
                ^^^^^^^^^^^^^^^^^^^^^ RSpec/SingleArgumentMessageChain: Use `receive` instead of calling `receive_message_chain` with a single argument.
end

before do
  foo.stub_chain(:one) { :two }
      ^^^^^^^^^^ RSpec/SingleArgumentMessageChain: Use `stub` instead of calling `stub_chain` with a single argument.
end

def stub_compute_management_client(user_supplied_value)
  allow(compute_management_client.virtual_machine_extensions).to receive_message_chain(
                                                                 ^^^^^^^^^^^^^^^^^^^^^ RSpec/SingleArgumentMessageChain: Use `receive` instead of calling `receive_message_chain` with a single argument.
    create_or_update: "create_or_update"
  ).and_return(stub_vm_extension_create_response(user_supplied_value))
end

it "reports single-key hash arguments" do
  allow(object).to receive_message_chain(:msg1 => :return_value)
                   ^^^^^^^^^^^^^^^^^^^^^ RSpec/SingleArgumentMessageChain: Use `receive` instead of calling `receive_message_chain` with a single argument.
end

it "reports dotted string hash keys" do
  allow(object).to receive_message_chain("msg1.msg2.msg3.msg4" => :return_value)
                   ^^^^^^^^^^^^^^^^^^^^^ RSpec/SingleArgumentMessageChain: Use `receive` instead of calling `receive_message_chain` with a single argument.
end

before do
  allow(controller).to receive_message_chain "forum.moderator?" => false
                       ^^^^^^^^^^^^^^^^^^^^^ RSpec/SingleArgumentMessageChain: Use `receive` instead of calling `receive_message_chain` with a single argument.
end

before do
  controller.stub_chain("admin?" => true)
             ^^^^^^^^^^ RSpec/SingleArgumentMessageChain: Use `stub` instead of calling `stub_chain` with a single argument.
end
