using size_t = decltype(sizeof(0));

size_t f() {
  return size_t();
}

int main() {
  return f();
}
