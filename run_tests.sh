#!/bin/bash

BASE_URL="http://127.0.0.1:8080"
ERRORS=0

echo "Starting localhost tests..."

# Helper function to check HTTP status codes
check_status() {
    local expected_status=$1
    local method=$2
    local url=$3
    local extra_args=$4

    # Run curl, suppress output, and just grab the HTTP status code
    STATUS=$(eval curl -s -o /dev/null -w \"%{http_code}\" -X $method $extra_args \"$url\")
    
    if [ "$STATUS" -eq "$expected_status" ]; then
        echo "✅ Passed: $method $url -> Got $STATUS"
    else
        echo "❌ Failed: $method $url -> Expected $expected_status, Got $STATUS"
        ERRORS=$((ERRORS + 1))
    fi
}

echo "--- Testing Error Pages ---"
# 403 Forbidden (Directory traversal or forbidden file check)
touch www/forbidden.html
chmod 000 www/forbidden.html
check_status 403 GET "$BASE_URL/forbidden.html"

# 404 Not Found
check_status 404 GET "$BASE_URL/nope"

# 405 Method Not Allowed (Assuming root / doesn't take POST)
check_status 405 POST "$BASE_URL/"

# 413 Payload Too Large
dd if=/dev/zero of=bigfile bs=1M count=2 2>/dev/null
# We expect the server to reject this big payload (Assuming 413 status)
check_status 413 POST "$BASE_URL/" "-H 'Content-Length: 2000000' --data-binary @bigfile"
rm -f bigfile www/forbidden.html

echo "--- Testing CGI ---"
# Unchunked GET
check_status 200 GET "$BASE_URL/cgi-bin/hello.py?x=1"
# Unchunked POST
check_status 200 POST "$BASE_URL/cgi-bin/hello.py" "--data hi"

# Chunked POST
printf "11\r\nchunked test data\r\n0\r\n\r\n" > chunk_test.txt
check_status 200 POST "$BASE_URL/cgi-bin/hello.py" "-H 'Transfer-Encoding: chunked' --data-binary @chunk_test.txt"
rm chunk_test.txt

echo "--- Testing Uploads and File Integrity ---"
# Create a test file
echo "Hello from CI" > test_upload.txt
# Upload it using multipart form data (you might need to adjust the route to match how your server handles form uploads)
curl -s -F "file=@test_upload.txt" "$BASE_URL/upload" > /dev/null

# Try to download the uploaded file and verify its contents
curl -s "$BASE_URL/uploads/test_upload.txt" -o downloaded.txt

# Diff returns 0 if files match
if diff test_upload.txt downloaded.txt > /dev/null; then
    echo "✅ Passed: File Upload & Download Integrity"
else
    echo "❌ Failed: Uploaded file does not match downloaded file"
    ERRORS=$((ERRORS + 1))
fi
rm test_upload.txt downloaded.txt

echo "--- Testing Routing and Deletion ---"
check_status 200 GET "$BASE_URL/"
# Redirect check: if /old redirects to /new, curl -L follows it. We check that it eventually hits a 200.
check_status 200 GET "$BASE_URL/old" "-L"

if [ $ERRORS -gt 0 ]; then
    echo "--------------------------"
    echo "❌ Tests Failed: $ERRORS errors found."
    exit 1
else
    echo "--------------------------"
    echo "🎉 All automated tests passed successfully!"
    exit 0
fi