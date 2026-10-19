// expect exit 0
int alive, arguments, copies;
struct D {
  D() { ++arguments; }
  ~D() noexcept(false) { --arguments; }
};
struct C {
  int value;
  C() : value(3) { ++alive; }
  C(const C& other, D = D()) : value(other.value) {
    if (++copies == 2) throw 7;
    ++alive;
  }
  ~C() noexcept { --alive; }
};
int main() {
  { C a[2]; try { auto f = [a] { return a[0].value; }; return f(); }
    catch (int n) { if (n != 7 || alive != 2 || arguments) return 1; } }
  if (alive) return 2;
  copies = 0;
  { C a[9]; try { auto f = [a] { return a[0].value; }; return f(); }
    catch (int n) { if (n != 7 || alive != 9 || arguments) return 3; } }
  return alive + arguments;
}
