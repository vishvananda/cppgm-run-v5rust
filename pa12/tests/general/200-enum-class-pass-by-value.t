enum class E {
  V = 8
};

int f(E e) {
  return static_cast<int>(e);
}

int main() {
  return f(E::V) - 8;
}
