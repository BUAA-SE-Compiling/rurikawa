var lost = require("lost");
var CompressionPlugin = require("compression-webpack-plugin");

module.exports = {
  plugins: [
    new CompressionPlugin({
      filename: "[path].gz",
      algorithm: "gzip",
      test: /\.js$|\.css$|\.html$/,
      threshold: 8192,
      minRatio: 0.8,
    }),
    new CompressionPlugin({
      filename: "[path].br",
      algorithm: "brotliCompress",
      test: /\.(js|css|html|svg)$/,
      compressionOptions: {
        level: 14,
      },
      threshold: 8192,
      minRatio: 0.8,
    }),
  ],
  module: {
    rules: [
      {
        test: /\.css$/,
        use: [
          {
            loader: "postcss-loader",
            options: {
              plugins: [lost],
            },
          },
        ],
      },
      {
        test: /\.less$/,
        use: [
          // "style-loader",
          // {
          //   loader: "postcss-loader",
          // },
          // { loader: "css-loader" },
          "less-loader",
        ],
      },
    ],
  },
};
