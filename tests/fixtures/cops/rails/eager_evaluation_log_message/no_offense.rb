Rails.logger.debug { "The time is #{Time.zone.now}." }
Rails.logger.debug "Simple string without interpolation"
Rails.logger.info "Info: #{user.name}"
Rails.logger.debug "plain message"
logger.debug "not Rails.logger"
puts "not a logger call"
