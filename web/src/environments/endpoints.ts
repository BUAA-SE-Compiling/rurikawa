export const endpoints = {
  account: {
    login: 'account/login',
    register: 'account/register',
    editPassword: 'account/edit/password',
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
    getJudgerStat: 'status/judger',
    getCode: 'admin/code',
    dumpSuiteJobs: (id: string) => `admin/suite/${id}/dump_jobs`,
    dumpSuiteAllJobs: (id: string) => `admin/suite/${id}/dump_all_jobs`,
    judgerRegisterToken: 'admin/judger/register-token',
  },
  dashboard: {
    get: 'dashboard',
  },
  testSuite: {
    query: 'tests',
    get: (id: string) => `tests/${id}`,
    getJobs: (id: string) => `tests/${id}/jobs`,
    setFile: (id: string) => `tests/${id}/file`,
    setVisibility: (id: string) => `tests/${id}/visibility`,
    put: (id: string) => `tests/${id}`,
    remove: (id: string) => `tests/${id}`,
    post: 'tests',
    ws: 'tests/ws?token=:token',
  },
  job: {
    get: 'job/:id',
    new: 'job',
    query: 'job',
  },
  file: {
    get: (filename: string) => `file/${filename}`,
  },
};
