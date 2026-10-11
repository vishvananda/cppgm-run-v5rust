enum class K { A, B };

int f(K k) {
  switch (k) {
    case K::A:
      return 1;
    case K::B:
      return 2;
  }
  return 0;
}

int main() {
  return f(K::B) - 2;
}
