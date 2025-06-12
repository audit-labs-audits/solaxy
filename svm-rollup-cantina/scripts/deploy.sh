#!/bin/bash

# This file can be used to trigger manual deploys of the SVM rollup server binary to AWS.
# It's also invoked as part of autodeploys in CI.

# These environment variables should be set when running the script:
# USER=
# HOST=
# DEPLOY_PATH=
# PRIVATE_KEY=
# BINARY_PATH=
# ROLLUP_CONFIG_PATH
# GENESIS_CONFIG_PATH

# Write the private key to a file
echo "$PRIVATE_KEY" > private_key.pem
chmod 600 private_key.pem

# Stop the service
ssh -o StrictHostKeyChecking=no -i private_key.pem $USER@$HOST 'sudo systemctl stop svm-rollup'

# Copy binary + config files
scp -o StrictHostKeyChecking=no -i private_key.pem $BINARY_PATH $USER@$HOST:$DEPLOY_PATH
scp -o StrictHostKeyChecking=no -i private_key.pem $ROLLUP_CONFIG_PATH $USER@$HOST:$DEPLOY_PATH
scp -o StrictHostKeyChecking=no -i private_key.pem -r $GENESIS_CONFIG_PATH $USER@$HOST:$DEPLOY_PATH

# Move files and set permissions
ssh -o StrictHostKeyChecking=no -i private_key.pem $USER@$HOST "
  sudo chown -R $USER:$USER $DEPLOY_PATH &&
  sudo chmod +x $DEPLOY_PATH/svm-rollup &&

  if [ ! -f /etc/systemd/system/svm-rollup.service ]; then
    echo 'Service file missing' >&2
    exit 1
  fi &&

  sudo systemctl daemon-reload &&
  sudo systemctl enable svm-rollup &&
  sudo systemctl restart svm-rollup
"

rm private_key.pem