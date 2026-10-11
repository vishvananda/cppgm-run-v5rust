// N3485 [except.handle]: unnamed catch parameters have their own lifetime.
int live, drops, bad, trace;
struct Error {
  Error() { ++live; }
  Error(Error const&) { ++live; }
  ~Error() { --live; trace = trace * 10 + 9; }
};
struct Guard {
  int expected;
  Guard(int count) : expected(count) {}
  ~Guard() { ++drops; if (live != expected) bad = 1; trace = trace * 10 + 1; }
};
void single(int mode) {
  try { throw Error(); }
  catch (Error) {
    trace = 0;
    Guard guard(2);
    if (mode == 0) {
      try { throw 7L; } catch (bool) {}
    } else if (mode == 1) {
      try { throw; } catch (bool) {}
    } else {
      try { throw 1; } catch (int) { throw 7L; }
    }
  }
}
void nested() {
  try { throw Error(); }
  catch (Error) {
    try { throw Error(); }
    catch (Error) {
      trace = 0;
      Guard guard(4);
      try { throw 7L; } catch (bool) {}
    }
  }
}
int main() {
  for (int mode = 0; mode != 4; ++mode) {
    drops = bad = trace = 0;
    try {
      if (mode == 3) nested(); else single(mode);
      return 1;
    } catch (long value) {
      if (mode == 1 || value != 7 || live || bad || drops != 1 ||
          trace != (mode == 3 ? 19999 : 199)) return 2;
    } catch (Error const&) {
      if (mode != 1 || live != 1 || bad || drops != 1 || trace != 19) return 3;
    }
    if (live || bad || drops != 1 || trace != (mode == 3 ? 19999 : 199))
      return 4;
  }
  return 0;
}
