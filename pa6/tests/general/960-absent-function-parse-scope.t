void ordinary() noexcept(noexcept(({int ignored=0;ignored;}))) {int local=1;}
void following(int parameter) {int local=parameter;}
struct Example {
  int value;
  Example() noexcept(noexcept(({int ignored=0;ignored;}))) : value(1) {}
  int read() noexcept(noexcept(({int ignored=0;ignored;}))) {return value;}
};
