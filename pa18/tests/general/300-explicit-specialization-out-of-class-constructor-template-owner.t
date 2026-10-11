namespace explicit_specialization_constructor_owner {

template <class T>
struct box;

template <>
struct box<void> {
  int result;

  template <class Fn>
  explicit box(Fn&&);
};

template <class Fn>
box<void>::box(Fn&& fn) : result(0) {
  int value = 0;
  fn(value);
  result = value;
}

void target(int& value) {
  value = 7;
}

int run() {
  box<void> value(target);
  return value.result == 7 ? 0 : 1;
}

}  // namespace explicit_specialization_constructor_owner

int main() {
  return explicit_specialization_constructor_owner::run();
}
