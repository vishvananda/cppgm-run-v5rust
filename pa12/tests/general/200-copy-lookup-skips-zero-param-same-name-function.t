// Copy/move-constructor lookup must ignore a zero-parameter same-name function.
struct A {};
void A();

struct A pass(struct A&& value) {
  return static_cast<struct A&&>(value);
}

int main() {
  struct A value;
  pass(static_cast<struct A&&>(value));
  return 0;
}
// VALIDATION: compile-pass
