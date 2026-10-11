int main() {
  int x = 1;
  auto f = [&]() -> int {
    try {
      throw 7;
    } catch(const int &error) {
      return x + error;
    }
  };
  return f() - 8;
}
