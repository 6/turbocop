@current_user ||= User.find_by!(id: session[:user_id])
current_user ||= User.find_by(id: session[:user_id])
@current_user ||= User.find_by(id: session[:user_id]) || User.anonymous
@current_user ||= session[:user_id] ? User.find_by(id: session[:user_id]) : nil
@current_user = User.find_by(id: session[:user_id])
@current_user &&= User.find_by(id: session[:user_id])
