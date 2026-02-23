User.find(id)
User.find_by(name: name)
User.where(name: 'Gabe').take!
User.find_by!(name: name)
User.where(id: id).first
User.find_by_id(id)
User.find_by!(pubsub_token: token, id: params[:user_id])
account.copilot_threads.find_by!(id: params[:id], user: current_user)
