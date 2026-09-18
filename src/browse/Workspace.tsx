export function BrowserWorkspace({ sessionId: _sessionId, onAuthLost: _onAuthLost }: { sessionId: string; onAuthLost: () => void }) {
  return <main className="browse-content"><h1>Following</h1><p role="status">Your Twitch account is connected.</p></main>;
}
