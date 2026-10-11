struct Sink
{
  Sink() : value(0) {}
  Sink(const Sink & other) : value(other.value + 100) {}

  template<class F>
  Sink(F f) : value(f(6)) {}

  int value;
};

int main()
{
  Sink sink = [](int x) { return x + 1; };
  return sink.value == 7 ? 0 : 1;
}

// VALIDATION: compile-pass
