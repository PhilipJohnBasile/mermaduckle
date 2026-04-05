import json
import urllib.request
import urllib.error
import sys

URL = 'http://127.0.0.1:3001/api/settings/api-keys'
# Seeded dev key present in the codebase
API_TOKEN = 'sk_dev_qwe456**********************'

def main():
    payload = json.dumps({"name": "local-client"}).encode('utf-8')
    req = urllib.request.Request(URL, data=payload, method='POST')
    req.add_header('Authorization', f'Bearer {API_TOKEN}')
    req.add_header('Content-Type', 'application/json')
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            body = resp.read().decode('utf-8')
            print(body)
    except urllib.error.HTTPError as e:
        try:
            err = e.read().decode('utf-8')
        except Exception:
            err = str(e)
        print(f'HTTPError {e.code}: {err}', file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f'Error: {e}', file=sys.stderr)
        sys.exit(1)

if __name__ == '__main__':
    main()
