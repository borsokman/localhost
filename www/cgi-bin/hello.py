#!/usr/bin/env python3
import os, sys

def main():
    method = os.environ.get("REQUEST_METHOD", "GET")
    qs = os.environ.get("QUERY_STRING", "")
    body = sys.stdin.read()
    print("Status: 200 OK")
    print("Content-Type: text/plain; charset=utf-8")
    print()
    print(f"Hello from CGI!\nMethod: {method}\nQuery: {qs}\nBody: {body}")

if __name__ == "__main__":
    main()