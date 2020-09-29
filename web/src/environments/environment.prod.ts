export const environment = {
  production: true,
  endpointBase: () => '/api/v1/',
  websocketBase: () => {
    let ws = new URL('/api/v1/', window.location.href);
    ws.protocol = ws.protocol.replace('http', 'ws');
    return ws;
  },
};
