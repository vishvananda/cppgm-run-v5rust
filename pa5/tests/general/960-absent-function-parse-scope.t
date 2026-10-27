void ordinary() noexcept(noexcept(({int ignored=0;ignored;}))) {}
void following(int parameter) { int local=parameter; }
void tried() noexcept(noexcept(({int ignored=0;ignored;}))) try {} catch(...) {}
struct Example {
  int value;
  Example() noexcept(noexcept(({int ignored=0;ignored;}))) : value(1) {}
  int read() noexcept(noexcept(({int ignored=0;ignored;}))) {return value;}
};
template<class T> void unused() noexcept(noexcept(({int ignored=0;ignored;}))) {}
