import json, os, subprocess, threading, time, urllib.request

CHANNEL = '/Users/rick.koh99/Desktop/voice-bridge/spikes/s1/voice/channel.mjs'
HUB = 'http://127.0.0.1:8795'


class Session:
    def __init__(self, sid, cwd):
        self.sid, self.lines = sid, []
        self.proc = subprocess.Popen(['node', CHANNEL], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True,
                                     env={**os.environ, 'CLAUDE_CODE_SESSION_ID': sid, 'CLAUDE_PROJECT_DIR': cwd})
        threading.Thread(target=lambda: [self.lines.append(json.loads(l)) for l in self.proc.stdout], daemon=True).start()
        self.rpc({'jsonrpc': '2.0', 'id': 1, 'method': 'initialize', 'params': {'protocolVersion': '2025-06-18', 'capabilities': {}, 'clientInfo': {'name': 'test', 'version': '0'}}})
        self.wait(lambda m: m.get('id') == 1)
        self.rpc({'jsonrpc': '2.0', 'method': 'notifications/initialized'})

    def rpc(self, msg):
        self.proc.stdin.write(json.dumps(msg) + '\n'); self.proc.stdin.flush()

    def wait(self, pred, timeout=4):
        end = time.time() + timeout
        while time.time() < end:
            for m in self.lines:
                if pred(m): return m
            time.sleep(0.05)

    def received(self):
        return [m['params']['content'] for m in self.lines if m.get('method') == 'notifications/claude/channel']


def post(path, body):
    req = urllib.request.Request(HUB + path, data=body.encode(), method='POST')
    with urllib.request.urlopen(req, timeout=3) as r: return r.status


a, b = Session('test-a', '/tmp/folder-a'), Session('test-b', '/tmp/folder-b')
time.sleep(0.5)
listed = {s['id']: s['cwd'] for s in json.load(urllib.request.urlopen(HUB + '/sessions'))}
results = {'both sessions registered with their folders': listed.get('test-a') == '/tmp/folder-a' and listed.get('test-b') == '/tmp/folder-b'}

post('/send?session=test-a', 'for a')
post('/send?session=test-b', 'for b')
time.sleep(0.5)
results['each message reaches only its session'] = a.received() == ['for a'] and b.received() == ['for b']

events = []
stream = urllib.request.urlopen(HUB + '/events', timeout=3)
threading.Thread(target=lambda: [events.append(json.loads(l[6:])) for l in stream if l.startswith(b'data: ')], daemon=True).start()
time.sleep(0.2)
b.rpc({'jsonrpc': '2.0', 'id': 2, 'method': 'tools/call', 'params': {'name': 'reply', 'arguments': {'text': 'reply from b'}}})
b.wait(lambda m: m.get('id') == 2)
time.sleep(0.3)
results['reply is tagged with its session'] = events == [{'session': 'test-b', 'text': 'reply from b'}]

a.proc.terminate(); a.proc.wait()
time.sleep(0.5)
results['a closed session drops off the list'] = [s['id'] for s in json.load(urllib.request.urlopen(HUB + '/sessions'))] == ['test-b']
b.proc.terminate()

for k, v in results.items(): print('PASS' if v else 'FAIL', k)
