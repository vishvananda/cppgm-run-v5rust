struct empty {};

struct pair_value {
  int first;
  int second;
};

template<class T, class... Args>
int construct(Args... args) {
  T value{args...};
  (void)value;
  return 0;
}

int main() {
  return construct<empty>() + construct<pair_value>(1, 2);
}
