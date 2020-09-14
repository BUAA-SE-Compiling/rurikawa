export const endpoints = {
  account: {
    login: 'account/login',
    register: 'account/register',
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
  },
  job: {
    get: 'job/:id',
    new: 'job',
    query: 'job',
  },
};
