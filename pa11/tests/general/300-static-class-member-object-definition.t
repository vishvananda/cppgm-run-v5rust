struct Cell {
  int value;
  Cell() : value(7) {}
};

struct Holder {
  static Cell cell;
};

Cell Holder::cell;

int main() {
  return Holder::cell.value - 7;
}
