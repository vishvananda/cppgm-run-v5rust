// expect exit 0: shared return cleanups retain enclosing handler contexts.
int live, drops, fail;
struct G {
  int id;
  G(int value) : id(value) { ++live; }
  ~G() noexcept(false) { --live; ++drops; if (id == fail) throw id; }
};
struct R { int value; R(int n) : value(n) {} ~R() noexcept(false) {} };
R run(int choice) {
  G outer(1);
  try {
    G inner(2);
    if (choice) return R(10);
    return R(20);
  } catch (int value) { return R(30 + value); }
}
int main() {
  for (int choice = 0; choice != 2; ++choice) {
    for (int mode = 0; mode != 3; ++mode) {
      drops = 0; fail = mode;
      try {
        R value = run(choice);
        if (mode == 1 || live || drops != 2 ||
            value.value != (mode == 2 ? 32 : choice ? 10 : 20)) return 1;
      } catch (int value) { if (mode != 1 || value != 1 || live || drops != 2) return 2; }
    }
  }
  return 0;
}
