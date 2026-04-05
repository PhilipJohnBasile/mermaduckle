// Console snippet: paste into browser DevTools Console to set a one-time API key
// Replace <YOUR_KEY> with the raw one-time API key returned by the server.
// After running this, the SPA will pick up the key from localStorage and reload.

// Example usage:
// localStorage.setItem('apiKey', '<YOUR_KEY>');
// window.apiKey = '<YOUR_KEY>'; // optional - set global used by SPA
// location.reload();

// One-liner for convenience (replace the placeholder):
// (function(k){ localStorage.setItem('apiKey', k); window.apiKey = k; location.reload(); })('<YOUR_KEY>');

// Helpful note:
// - The raw API key is shown only once when created or rotated by the server.
// - For local development you can also run `python tools/create_api_key.py` to request a key from
//   a locally-running server that has the seeded dev admin token.
