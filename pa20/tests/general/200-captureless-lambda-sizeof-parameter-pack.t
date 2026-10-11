template<class... Args>
int g() {
  auto f = [](Args... args2) { return sizeof...(args2); };
  (void)f;
  return 0;
}

int main() { return g<int, char>(); }
