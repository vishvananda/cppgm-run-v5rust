// N3485 6.6.3p2: preserve a shared return value before destroying
// full-expression temporaries and then automatic objects; expect exit 0
int destroyed;
struct Temporary {
  int value;
  Temporary(int n) : value(n) {}
  ~Temporary() noexcept { ++destroyed; }
};
int read(const Temporary& value) { return value.value; }
int choose(bool first) {
  Temporary local(9);
  if (first) return read(Temporary(7));
  return read(Temporary(8));
}
int main() {
  return choose(true) != 7 || choose(false) != 8 || destroyed != 4;
}
