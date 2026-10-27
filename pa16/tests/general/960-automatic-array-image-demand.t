constexpr int hidden_array() {
  int hidden_values[3] = {1, 2, 3};
  return hidden_values[0];
}
constexpr int held_value = hidden_array();

int unreachable_array() {
  return 0;
  int dead_values[3] = {4, 5, 6};
}

struct Base {
  int base;
  Base() : base(0) {}
};
struct BothEntries : virtual Base {
  int value;
  BothEntries(int index) {
    int live_values[3] = {1, 2, 3};
    value = live_values[index];
  }
};

int main() {
  BothEntries object(1);
  return held_value != 1 || object.value != 2 || unreachable_array();
}
