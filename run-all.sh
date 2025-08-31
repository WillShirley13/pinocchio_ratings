#! /bin/bash

# start local validator
echo "Starting local validator..."
solana-test-validator --reset &

# wait for validator to start
sleep 10

# build program
echo "Building program..."
cargo build-sbf

# deploy program
echo "Deploying program..."
solana program deploy target/deploy/pinocchio_ratings.so

# run tests
echo "Running tests..."
npm test

# kill validator
echo "Killing validator..."
pkill -f solana-test-validator