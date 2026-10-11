namespace explicit_specialization_member_owner {

template <class T>
struct box;

template <>
struct box<void> {
  template <class U>
  int apply(U);
};

template <class U>
int box<void>::apply(U) {
  return sizeof(U);
}

int run() {
  box<void> value;
  return value.apply<int>(0) == sizeof(int) ? 0 : 1;
}

}  // namespace explicit_specialization_member_owner

int main() {
  return explicit_specialization_member_owner::run();
}
