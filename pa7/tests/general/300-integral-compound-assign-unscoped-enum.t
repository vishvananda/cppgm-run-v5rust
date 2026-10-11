enum State {
  constructed = 1,
  future_attached = 2,
  ready = 4
};

unsigned attach(unsigned state) {
  state |= future_attached;
  state += constructed;
  return state;
}
