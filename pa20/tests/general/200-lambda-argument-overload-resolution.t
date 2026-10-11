template<class F>
int call_with_int(F fn, int value) {
  return fn(value);
}

template<class F>
int choose(F fn, int value) {
  return call_with_int(fn, value);
}

template<class F>
int choose(F fn, long value) {
  return call_with_int(fn, static_cast<int>(value + 10));
}

int main() {
  return choose([](int value) { return value; }, 7) == 7 ? 0 : 1;
}
