@current_user ||= User.find_by!(id: session[:user_id])
current_user ||= User.find_by(id: session[:user_id])
@current_user ||= User.find_by(id: session[:user_id]) || User.anonymous
@current_user ||= session[:user_id] ? User.find_by(id: session[:user_id]) : nil
@current_user = User.find_by(id: session[:user_id])
@current_user &&= User.find_by(id: session[:user_id])
@follow_request ||= FollowRequest.find_by(target_account: @account, uri: uri) unless uri.nil?
@follow ||= Follow.find_by(target_account: @account, uri: uri) if active?
