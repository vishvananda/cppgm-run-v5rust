namespace N {
  int g(int x) { return x + 2; }
}

int main() {
  return N::g(3);
}
