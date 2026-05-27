import json
import browser_cookie3
import urllib.request

cookies = {c.name: c.value for c in browser_cookie3.chrome(domain_name=".perplexity.ai")}

session_token = cookies.get("__Secure-next-auth.session-token", "")
cf_clearance = cookies.get("cf_clearance", "")

req = urllib.request.Request(
    "https://www.perplexity.ai/api/auth/session",
    headers={
        "Cookie": f"__Secure-next-auth.session-token={session_token}; cf_clearance={cf_clearance}",
        "User-Agent": "Mozilla/5.0",
    },
)
with urllib.request.urlopen(req) as resp:
    data = json.loads(resp.read())

csrf_token = data.get("user", {}).get("pplx_csrf_token", cookies.get("pplx-csrf-token", ""))

print(json.dumps({
    "session_token": session_token,
    "cf_clearance": cf_clearance,
    "csrf_token": csrf_token,
}))