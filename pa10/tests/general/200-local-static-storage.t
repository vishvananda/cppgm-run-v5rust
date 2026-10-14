int inits;
int next() { return ++inits; }
inline int shared() { static int n = next(); return ++n; }
int select(bool first) {
  if (first) { static int n[3] = {next(), next()}; return ++n[0]; }
  else { thread_local int n = next(); return ++n; }
}
int main() { return shared() + shared() + select(true) + select(true) + select(false); }
