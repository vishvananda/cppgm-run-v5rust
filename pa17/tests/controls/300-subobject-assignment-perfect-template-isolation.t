// expect exit 0
struct Cell {
  int n;
  Cell& operator=(Cell const& other) noexcept { n = other.n; return *this; }
  template<class T> typename T::missing operator=(T&&);
};
struct Store { Cell cell; };
int main() {
  Store target{};
  const Store source = {{7}};
  target = source;
  return target.cell.n - 7;
}

