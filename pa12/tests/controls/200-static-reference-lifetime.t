// Static references retain the complete temporary, including shutdown lifetime.
int calls, alive, destroyed;
int next() { return ++calls + 40; }
struct Verify {
  ~Verify() { if (alive || destroyed != 3) __builtin_abort(); }
} verify;
struct Owner {
  int tag;
  long value;
  Owner(int n) : tag(n), value(100 + n) { ++alive; }
  ~Owner() {
    if (value != 100 + tag) __builtin_abort();
    --alive;
    ++destroyed;
  }
};
bool choose() { return calls == 9; }
const int& scalar = next();
const long& converted = next();
const long& member = Owner(2).value;
const long& selected = choose() ? Owner(3).value : Owner(4).value;
const long& selected_true = !choose() ? Owner(5).value : Owner(6).value;
int main() {
  volatile long clobber[8] = {99, 98, 97, 96, 95, 94, 93, 92};
  return scalar != 41 || converted != 42 || calls != 2 ||
    member != 102 || selected != 104 || selected_true != 105 ||
    alive != 3 || destroyed != 0 ||
    clobber[7] != 92;
}
