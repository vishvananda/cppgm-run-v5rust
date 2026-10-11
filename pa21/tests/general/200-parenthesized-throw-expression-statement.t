int main() {
  try {
    (throw 7);
  } catch (int x) {
    return x - 7;
  }
}
