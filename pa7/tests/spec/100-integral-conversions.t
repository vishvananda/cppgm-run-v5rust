// N3485 focus: 4.7 [conv.integral] integral conversions
long h(long x) { return x; }
long f() { short y = 0; long z = y; return h(y); }
