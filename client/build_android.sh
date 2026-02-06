cargo apk build --target aarch64-linux-android --lib
echo "done; uploading..."
mv ../target/debug/apk/client.apk ../target/debug/apk/skat-client.apk
scp ../target/debug/apk/skat-client.apk $DEV_SERVER
