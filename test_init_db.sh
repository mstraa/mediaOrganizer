#!/bin/bash

# Test script for the updated init-db command

echo "Testing init-db command with single directory parameter..."
echo

# Create a test directory structure
TEST_DIR="/tmp/test_media_organizer"
rm -rf "$TEST_DIR"
mkdir -p "$TEST_DIR/photos"
mkdir -p "$TEST_DIR/videos"

# Create some dummy files
echo "Creating test files..."
for i in {1..5}; do
    touch "$TEST_DIR/photos/photo$i.jpg"
    touch "$TEST_DIR/videos/video$i.mp4"
done

# Run init-db command
echo "Running init-db command..."
echo "Command: cargo run -- init-db -d $TEST_DIR"
cargo run --manifest-path=media-organizer/Cargo.toml -- init-db -d "$TEST_DIR"

# Check if database was created
echo
echo "Checking for database file..."
if [ -f "$TEST_DIR/db.mediaorg" ]; then
    echo "✅ Database file created successfully at: $TEST_DIR/db.mediaorg"
    ls -la "$TEST_DIR/db.mediaorg"
else
    echo "❌ Database file not found!"
fi

echo
echo "Test complete. Cleaning up..."
rm -rf "$TEST_DIR"