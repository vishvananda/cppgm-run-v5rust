struct value {
  value(int, int) {}
};

struct holder {
  value first;
  int second;
};

int main() {
  holder x{value{1, 2}, 0};
  return x.second;
}
