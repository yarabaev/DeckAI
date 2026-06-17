// Caps match current brotli-compressed bundle sizes after the
// Noto Sans SC / TC / JP + Pretendard inlines landed. The fonts
// budget is large because the four CJK families ship as inlined
// base64 .js modules for layout-time text measurement — long-term
// follow-up is to switch to on-demand loading, after which these
// numbers should drop back into the single-digit MB range.
export default [
  {
    name: "fonts",
    path: "dist/calcYogaLayout/fonts/*.js",
    limit: "50 MB",
  },
  {
    name: "icon data",
    path: "dist/icons/iconData.js",
    limit: "100 KB",
  },
  {
    name: "total",
    path: "dist/**/*.js",
    limit: "50 MB",
  },
];
