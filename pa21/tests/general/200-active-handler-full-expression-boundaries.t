// N3485 [class.temporary], [except.handle], [except.throw]: temporaries
// finish before the active handler releases its caught exception object.
int exceptions, arguments, drops, bad;
struct Error {
  Error() { ++exceptions; }
  Error(Error const&) { ++exceptions; }
  ~Error() { --exceptions; }
};
struct Argument {
  Argument(int) { ++arguments; }
  ~Argument() {
    --arguments;
    ++drops;
    if (exceptions != 1) bad = 1;
  }
};
int use(Argument const&, bool fail) {
  if (fail) throw 7;
  return 0;
}
int probe(int mode) {
  try { throw Error(); }
  catch (Error const&) {
    if (mode == 0) return use(Argument(1), true);
    if (mode == 1) { use(Argument(1), true); return 3; }
    if (mode == 2) { int result = use(Argument(1), true); return result; }
    if (mode == 3) { if (use(Argument(1), true)) return 3; return 4; }
    return use(Argument(1), false);
  }
}
int main() {
  for (int mode = 0; mode != 5; ++mode) {
    drops = 0;
    try {
      if (probe(mode) || mode != 4) return 2;
    } catch (int number) {
      if (number != 7 || mode == 4) return 3;
    }
    if (exceptions || arguments || drops != 1 || bad) return 1;
  }
  return 0;
}
