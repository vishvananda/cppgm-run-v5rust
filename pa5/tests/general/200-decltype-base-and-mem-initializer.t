namespace n {
struct base {};
}

struct derived : decltype(n::base()) {
  derived() : decltype(n::base())() {}
};
