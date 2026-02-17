User.find(id)
User.find_by(name: name)
User.where(name: 'Gabe').take!
User.find_by!(name: name)
User.where(id: id).first
User.find_by_id(id)
