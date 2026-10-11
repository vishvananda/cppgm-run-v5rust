int take(double x) { return x == 2.0 ? 0 : 1; }
struct holder {
  double&& ref;
  holder(double&& x) : ref(static_cast<double&&>(x)) {}
};
int main() {
  double x = 2.0;
  holder h(static_cast<double&&>(x));
  return take(static_cast<double&&>(h.ref));
}
