// N3485 [except.ctor], [except.handle]: destroy each intervening prefix once.
int live, drops, trace;
struct Guard {
  int id;
  Guard(int value) : id(value) { ++live; }
  ~Guard() { --live; ++drops; trace = trace * 10 + id; }
};
int main() {
  for (int mode = 0; mode != 2; ++mode) {
    drops = trace = 0;
    try {
      Guard outer(1);
      try {
        Guard middle(2);
        try { throw 7; }
        catch (int) { if (mode) throw 8L; throw; }
      } catch (bool) { return 1; }
    } catch (int value) {
      if (mode || value != 7 || live || drops != 2 || trace != 21) return 2;
    } catch (long value) {
      if (!mode || value != 8 || live || drops != 2 || trace != 21) return 3;
    }
  }
  return 0;
}
