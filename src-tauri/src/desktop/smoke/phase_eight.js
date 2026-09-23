// Synthetic public browsing DTOs only. Settings and desktop operations still
// invoke Rust; authentication and Twitch HTTP never access a real account.
(() => {
  import("/src/lib/ipc.ts").then(({ api }) => {
  const invoke = window.__TAURI_INTERNALS__.invoke;
  const page = items => ({ items, cursor: null, freshness: "network", ageSeconds: 0, warnings: [] });
  const channel = { broadcasterId: "123", login: "synthetic", displayName: "Synthetic Channel", imageUrl: null, followedAt: null, liveState: "offline", title: null, categoryName: null, language: null };
  const category = { id: "42", name: "Synthetic Category", imageUrl: null };
  const synthetic = (command, args, options) => {
    if (command === "auth_status") return Promise.resolve({ phase: "authenticated", sessionId: "987654", user: { id: "987654", login: "synthetic", scopes: [], expiresIn: 3600 }, authorization: null, error: null, credentialStorage: "test" });
    if (command === "auth_account") return Promise.resolve({ id: "987654", login: "synthetic", displayName: "Synthetic Viewer", profileImageUrl: null });
    if (command === "get_team") return Promise.resolve({ id: "7", name: "synthetic-team", displayName: "Synthetic Team", description: "Synthetic native rendering fixture", imageUrl: null, members: page([channel]), memberCount: 1, limited: false });
    if (command === "lookup_channel") return Promise.resolve({ broadcasterId: "123", displayName: channel.displayName });
    if (command === "get_channel") return Promise.resolve({ channel, description: "Synthetic channel description", stream: null, freshness: "network", ageSeconds: 0, warnings: [] });
    if (command === "list_category_streams") return Promise.resolve({ category, streams: page([]) });
    if (command === "list_categories") return Promise.resolve(page([category]));
    if (["list_followed_streams", "list_followed_channels", "list_streams", "search_channels", "search_categories"].includes(command)) return Promise.resolve(page([]));
    if (command.startsWith("auth_")) return Promise.reject({ code: "cancelled" });
    return invoke(command, args, options);
  };
  for (const [method, command] of Object.entries({ authStatus: "auth_status", account: "auth_account", team: "get_team", lookupChannel: "lookup_channel", channel: "get_channel", category: "list_category_streams", categories: "list_categories", followedStreams: "list_followed_streams", followedChannels: "list_followed_channels", streams: "list_streams", searchChannels: "search_channels", searchCategories: "search_categories" })) {
    api[method] = (...args) => synthetic(command, ...args);
  }
  window.__phaseEightReady = true;
  });
  return true;
})()
