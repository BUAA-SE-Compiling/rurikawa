var lost = require('lost');

module.exports = {
  module: {
    rules: [
      {
        test: /\.css$/,
        use: [
          {
            loader: 'postcss-loader',
            options: {
              plugins: [lost],
            },
          },
        ],
      },
      {
        test: /\.styl$/,
        use: [
          {
            loader: 'postcss-loader',
            options: {
              plugins: [lost],
            },
          },
          {
            loader: 'stylus-loader',
          },
        ],
      },
    ],
  },
};
