void rethrow() {
  throw;
}

int f() {
  try {
    throw 3;
  } catch (int) {
    rethrow();
    return 1;
  }
}

int main() {
  try {
    return f();
  } catch (int x) {
    return x - 3;
  }
}
