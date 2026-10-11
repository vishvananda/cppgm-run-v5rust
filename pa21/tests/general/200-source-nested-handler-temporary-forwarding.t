// N3485 [class.temporary], [except.handle]: destroy arguments before catches.
int live, drops, bad;
struct Error {
  Error() { ++live; }
  Error(Error const&) { ++live; }
  ~Error() { --live; }
};
struct Argument {
  Argument(int) {}
  ~Argument() { ++drops; if (live != 1) bad = 1; }
};
int use(Argument const&) { throw 7L; }
int probe(bool nested_handler) {
  try { throw Error(); }
  catch (...) {
    if (nested_handler) {
      try { throw 1; }
      catch (...) { return use(Argument(1)); }
    } else {
      try { return use(Argument(1)); }
      catch (int) { return 2; }
    }
  }
}
int main() {
  for (int mode = 0; mode != 2; ++mode) {
    drops = bad = 0;
    try { probe(mode != 0); return 1; }
    catch (long value) {
      if (value != 7 || live || drops != 1 || bad) return 2;
    }
  }
  return 0;
}
