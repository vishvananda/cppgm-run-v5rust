struct Sink
{
  Sink() : value(0) {}

  Sink & operator=(const Sink & other)
  {
    value = other.value + 100;
    return *this;
  }

  template<class F>
  Sink & operator=(F f)
  {
    value = f(6);
    return *this;
  }

  int value;
};

int main()
{
  Sink sink;
  sink = [](int x) { return x + 1; };
  return sink.value == 7 ? 0 : 1;
}

// VALIDATION: compile-pass
