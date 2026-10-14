// expect reject
// Signature friendship belongs only to the matching canonical function.
class Secret {
  typedef int Key;
  friend Key inspect(Key);
};
Secret::Key inspect(long n) { return n; }
int main() { return 0; }
