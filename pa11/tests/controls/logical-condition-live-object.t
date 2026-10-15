// expect exit 0
int alive;
int calls;
int step() { return ++calls; }
struct Guard {
  Guard() { ++alive; }
  ~Guard() { --alive; }
};
int check() {
  Guard guard;
  if (step() && step()) return alive != 1;
  return 1;
}
int main() { int result = check(); return result || alive || calls != 2; }
