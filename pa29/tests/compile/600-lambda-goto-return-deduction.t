template <class R, class F>
R apply(F f) {
  return f();
}

int f() {
  return apply<int>([]() {
    if (false)
      goto done;
  done:
    return 1;
  });
}

int main() { return f() != 1; }
