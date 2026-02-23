class MyJob < ApplicationJob
end

class ApplicationJob < ActiveJob::Base
end

class ProcessDataJob < ApplicationJob
end
