// N3485 [except.handle], [except.throw]: locals finish before active catches.
int live, drops, bad, trace;
struct Error {
  Error() { ++live; }
  Error(Error const&) { ++live; }
  ~Error() { --live; trace = trace * 10 + 9; }
};
struct Guard {
  Guard(int) {}
  ~Guard() { ++drops; if (live != 1) bad = 1; trace = trace * 10 + 1; }
};
struct RethrowGuard {
  ~RethrowGuard() noexcept(false) {
    ++drops;
    if (live != 1) bad = 1;
    trace = trace * 10 + 2;
    throw;
  }
};
void probe(int mode) {
  try { throw Error(); }
  catch (Error const&) {
    trace = 0;
    if (mode == 0) {
      Guard guard(1);
      try { throw 7L; } catch (int) {}
    } else if (mode == 1) {
      try { throw 1; }
      catch (...) {
        Guard guard(1);
        try { throw 7L; } catch (bool) {}
      }
    } else if (mode == 2) {
      Guard guard(1);
      try { throw; } catch (int) {}
    } else if (mode == 3) {
      Guard guard(1);
      RethrowGuard rethrow;
      return;
    } else {
      Guard guard(1);
      try { throw; } catch (Error const&) { if (live != 1) bad = 1; }
      if (live != 1) bad = 1;
    }
  }
}
int main() {
  for (int mode = 0; mode != 5; ++mode) {
    drops = bad = trace = 0;
    const int expected_drops = mode == 3 ? 2 : 1;
    const int expected_trace = mode == 3 ? 21 : 1;
    try { probe(mode); }
    catch (long value) {
      if (mode >= 2 || value != 7 || live || bad || drops != 1 || trace != 19)
        return 1;
    } catch (Error const&) {
      if ((mode != 2 && mode != 3) || live != 1 || bad ||
          drops != expected_drops || trace != expected_trace) return 2;
    }
    if (live || bad || drops != expected_drops || trace != expected_trace * 10 + 9) return 3;
  }
  return 0;
}
