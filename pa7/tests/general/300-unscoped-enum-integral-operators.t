enum State {
  constructed = 1,
  future_attached = 2,
  ready = 4
};

int f(State state) {
  return (~state & ready) + (state << 1) + (state + 1);
}

bool g(unsigned state) {
  return (state & constructed) != 0;
}
