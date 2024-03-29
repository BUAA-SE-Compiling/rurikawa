export const endpoints = {
  account: {
    login: 'account/login',
    register: 'account/register',
    editPassword: 'account/edit/password',
    wsToken: 'account/ws-token',
  },
  profile: {
    get: (id: string) => `profile/${id}`,
    put: (id: string) => `profile/${id}`,
    init: (id: string) => `profile/${id}/init`,
  },
  admin: {
    readInitStatus: 'admin/init',
    setInitAccount: 'admin/init',
    getJudgerStat: 'status/judger',
    getCode: 'admin/code',
    // TODO: move these paths to /tests/ & update to v2
    dumpSuiteJobs: (id: string) => `admin/suite/${id}/dump_jobs`,
    dumpSuiteAllJobs: (id: string) => `admin/suite/${id}/dump_all_jobs`,
    judgerRegisterToken: 'admin/judger/register-token',
    getUserInfo: (username: string) => `admin/user-info/${username}`,
    registerUser: `admin/register`,
    searchUserInfo: `admin/user-info`,
    editPassword: `admin/edit-password`,
    testSuite: {
      querySuiteJobs: (id: string) => `admin/tests/${id}/jobs`,
    },
  },
  status: {
    queue: 'status/job-queue',
    judger: 'status/judger',
    assembly: 'status/assembly',
  },
  announcement: {
    query: 'announcement',
    post: 'announcement',
    get: (id: string) => `announcement/${id}`,
    set: (id: string) => `announcement/${id}`,
    delete: (id: string) => `announcement/${id}`,
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
    get: (id: string) => `job/${id}`,
    new: 'job',
    query: 'job',
    respawn: (id: string) => `job/respawn/${id}`,
  },
  file: {
    get: (filename: string) => `file/${filename}`,
  },
};
