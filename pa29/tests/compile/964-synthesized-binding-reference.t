int main() {
  int values[2] = {1, 2};
  auto bitand [first, second] = values;
  auto and [third, fourth] = values;
  int rows[2][2] = {{1, 2}, {3, 4}};
  for (auto bitand [left, right] : rows) {
    first += left + right;
  }
  int bitand written = first;
  return written + second + third + fourth;
}
