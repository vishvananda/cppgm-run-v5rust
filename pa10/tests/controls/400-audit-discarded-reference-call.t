// expect exit 0: casting a reference-returning call to void evaluates only the call.
int value = 7;
int calls;
int& reference() { ++calls; return value; }
volatile int& volatile_reference() { ++calls; return value; }
int main() {
  (void)reference();
  (void)volatile_reference();
  static_cast<void>(reference());
  static_cast<void>(volatile_reference());
  return calls != 4;
}
