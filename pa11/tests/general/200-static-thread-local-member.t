struct X {
  static thread_local int n;
};

thread_local int X::n = 1;

int main() {
  return X::n - 1;
}
