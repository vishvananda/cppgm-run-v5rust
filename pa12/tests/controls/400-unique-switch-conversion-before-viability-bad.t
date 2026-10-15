// N3485 6.4.2p2: two visible integral conversion declarations are not
// unique even when only one implicit object argument would be viable.
// expect reject
struct State {
  operator int() &;
  operator int() &&;
};
void f(State& state) { switch (state) { default: break; } }
