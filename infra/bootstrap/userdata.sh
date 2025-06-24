# Runs on an Ubuntu 24.04 instance

# Add Docker's official GPG key:
sudo apt-get update
sudo apt-get install ca-certificates curl build-essential -y
sudo install -m 0755 -d /etc/apt/keyrings
sudo curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc
sudo chmod a+r /etc/apt/keyrings/docker.asc

# Add the repository to Apt sources:
echo \
  "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu \
  $(. /etc/os-release && echo "${UBUNTU_CODENAME:-$VERSION_CODENAME}") stable" | \
  sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
sudo apt-get update

# Install Docker components
sudo apt-get install docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin -y

# Install Rust - this approach is interactive and will lock user data
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Get Github PAT to download image from private repo
export CR_PAT=""
echo $CR_PAT | docker login ghcr.io -u USERNAME --password-stdin

# Pull RPC image
docker pull ghcr.io/developer-dao/rpc:latest

# Clone repo 
git clone git@github.com:Developer-DAO/rpc.git /opt/rpc
cd /opt/rpc 

# Set up project
#cargo build
#cargo install openssl sqlx-cli

docker compose up -d 
