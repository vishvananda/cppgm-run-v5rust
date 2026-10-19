// expect exit 0
int alive, copies;
bool select = true;
int step() { if (++copies == 2) throw 7; return copies; }
struct C {
  int value;
  C() : value(3) { ++alive; }
  C(const C& other, int = (select ? step() : step())) : value(other.value) { ++alive; }
  ~C() noexcept { --alive; }
};
int main() {
  try { C a[2]; auto f = [a] { return a[0].value; }; return f(); }
  catch (int n) { if (n != 7 || alive) return 1; }
  copies = 0;
  select = false;
  try { C a[9]; auto f = [a] { return a[0].value; }; return f(); }
  catch (int n) { if (n != 7 || alive) return 2; }
  return alive;
}
