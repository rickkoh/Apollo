import json, os, subprocess, threading, time, urllib.request, urllib.error

BRIDGE = '/Users/rick.koh99/Desktop/voice-bridge/bridge.mjs'
PORT = '8791'
BASE = f'http://127.0.0.1:{PORT}'

proc = subprocess.Popen(['node', BRIDGE], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                        env={**os.environ, 'VOICE_BRIDGE_PORT': PORT})
lines = []
threading.Thread(target=lambda: [lines.append(json.loads(l)) for l in proc.stdout], daemon=True).start()

def rpc(msg):
    proc.stdin.write(json.dumps(msg) + '\n'); proc.stdin.flush()

def wait_for(pred, timeout=5):
    end = time.time() + timeout
    while time.time() < end:
        for m in lines:
            if pred(m): return m
        time.sleep(0.05)
    return None

def http(method, path, body=None, headers=None):
    req = urllib.request.Request(BASE + path, data=body, method=method, headers=headers or {})
    try:
        with urllib.request.urlopen(req, timeout=3) as r: return r.status, r.read()
    except urllib.error.HTTPError as e: return e.code, b''

results = {}
rpc({'jsonrpc': '2.0', 'id': 1, 'method': 'initialize', 'params': {'protocolVersion': '2025-06-18', 'capabilities': {}, 'clientInfo': {'name': 'test', 'version': '0'}}})
init = wait_for(lambda m: m.get('id') == 1)
results['channel capability declared'] = 'claude/channel' in init['result']['capabilities'].get('experimental', {})
results['instructions present'] = bool(init['result'].get('instructions'))
rpc({'jsonrpc': '2.0', 'method': 'notifications/initialized'})
rpc({'jsonrpc': '2.0', 'id': 2, 'method': 'tools/list'})
tools = wait_for(lambda m: m.get('id') == 2)
results['reply tool listed'] = [t['name'] for t in tools['result']['tools']] == ['reply']
time.sleep(0.3)

status, page = http('GET', '/')
results['page served'] = status == 200 and b'Voice bridge' in page

status, _ = http('POST', '/send', b'click the Settle button')
note = wait_for(lambda m: m.get('method') == 'notifications/claude/channel')
results['POST becomes channel notification'] = status == 204 and note and note['params']['content'] == 'click the Settle button'

status, _ = http('POST', '/send', b'evil', {'Origin': 'https://evil.example'})
results['foreign origin refused'] = status == 403
status, _ = http('POST', '/send', b'evil', {'Host': f'attacker.example:{PORT}'})
results['foreign host refused'] = status == 403
results['refused posts not forwarded'] = sum(1 for m in lines if m.get('method') == 'notifications/claude/channel') == 1

events = []
try:
    stream = urllib.request.urlopen(BASE + '/events', timeout=2)
    results['stream opens before any reply'] = stream.status == 200
except OSError:
    stream = None
    results['stream opens before any reply'] = False
def sse():
    for raw in stream:
        if raw.startswith(b'data: '): events.append(json.loads(raw[6:])); return
if stream: threading.Thread(target=sse, daemon=True).start()
rpc({'jsonrpc': '2.0', 'id': 3, 'method': 'tools/call', 'params': {'name': 'reply', 'arguments': {'text': 'Clicked Settle.'}}})
reply = wait_for(lambda m: m.get('id') == 3)
time.sleep(0.3)
results['reply tool returns sent'] = reply['result']['content'][0]['text'] == 'sent'
results['reply reaches page stream'] = events == [{'text': 'Clicked Settle.'}]

proc.terminate()
for k, v in results.items(): print(('PASS' if v else 'FAIL'), k)
err = proc.stderr.read()
if err: print('stderr:', err)
