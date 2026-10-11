struct Table {
  int f() { return 1; }
  int f() const { return 2; }
};

struct Map {
  Table table;

  int g() const {
    return table.f();
  }
};

int main() {
  const Map m = {};
  return m.g() == 2 ? 0 : 1;
}
