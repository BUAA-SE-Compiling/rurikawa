export const endpoints = {
  account: {
    login: 'account/login',
    register: 'account/register',
    wsToken: 'account/ws-token',
  },
  profile: {
    get: (id: string) => `profile/${id}`,
    set: (id: string) => `profile/${id}`,
    init: (id: string) => `profile/${id}/init`,
  },
  admin: {
    readInitStatus: 'admin/init',
    setInitAccount: 'admin/init',
  },
  dashboard: {
    get: 'dashboard',
  },
  testSuite: {
    query: 'tests',
    get: 'tests/:id',
    getJobs: 'tests/:id/jobs',
    post: 'tests',
    ws: 'tests/ws?token=:token',
  },
  job: {
    get: 'job/:id',
    new: 'job',
    query: 'job',
  },
};
