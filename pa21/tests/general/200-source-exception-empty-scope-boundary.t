// N3485 [except.ctor]: an empty lexical scope still bounds unwinding.
int live, drops, trace;
struct Guard {
  int id;
  Guard(int value) : id(value) { ++live; }
  ~Guard() { --live; ++drops; trace = trace * 10 + id; }
};
int probe(int mode) {
  Guard outside(1);
  {
    try {
      if (mode == 0) {
        try { throw 7; }
        catch (int value) { if (value != 7 || live != 1 || drops) return 1; }
      } else if (mode == 1) {
        try { throw 7; } catch (long) { return 2; }
      } else if (mode == 2) {
        Guard inside(2);
        try { throw 7; } catch (long) { return 3; }
      } else {
        try { throw 7; } catch (int) { throw 8L; }
      }
    } catch (int value) {
      if (value != 7 || live != 1 || drops != (mode == 2) ||
          trace != (mode == 2 ? 2 : 0)) return 4;
    } catch (long value) {
      if (mode != 3 || value != 8 || live != 1 || drops) return 5;
    }
  }
  return live != 1;
}
int main() {
  for (int mode = 0; mode != 4; ++mode) {
    drops = trace = 0;
    if (probe(mode) || live || drops != (mode == 2 ? 2 : 1) ||
        trace != (mode == 2 ? 21 : 1)) return 1;
  }
  return 0;
}
