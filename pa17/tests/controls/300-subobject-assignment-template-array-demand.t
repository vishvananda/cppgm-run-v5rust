// expect exit 0
struct Cell {
  int n;
  template<class T>
  volatile Cell& operator=(T const& other) volatile noexcept(sizeof(T) > 0) {
    n = other.n + 1;
    return *this;
  }
};
struct Store { volatile Cell cells[9]; };
int main() {
  Store target{}, source{};
  source.cells[0].n = 4;
  static_assert(noexcept(target = source), "assignment specification");
  target = source;
  target = source;
  return target.cells[0].n == 5 && target.cells[8].n == 1 ? 0 : 1;
}

