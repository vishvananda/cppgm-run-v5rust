int main() {
  return []<class U>(U u, int y = 3) -> int {
    return u + y;
  }(1) - 4;
}
