// The abort builtin retains its runtime boundary through calls and addresses.
// expect exit 0
void fail_direct(int n) { if (n) __builtin_abort(); }
void fail_pointer(int n) {
  void (*abort_function)() = &__builtin_abort;
  if (n) abort_function();
}
int main() {
  fail_direct(0);
  fail_pointer(0);
  return 0;
}
