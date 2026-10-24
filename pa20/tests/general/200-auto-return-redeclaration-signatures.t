// Placeholder results belong to the shared function entity across declarations.
auto value();
auto value();
auto value() { return 3; }

struct Counter { auto get(); };
auto Counter::get() { return 4L; }

int object = 5;
auto& reference();
auto& reference() { return object; }

int main() {
  Counter counter;
  reference() += value();
  return object == 8 && counter.get() == 4 ? 0 : 1;
}
