template <class T, class Traits = T*>
struct first_like {
  typedef Traits traits_type;
};

template <class T, class Traits = const T*>
struct second_like {
  typedef Traits traits_type;
};

int pick(int*) {
  return 1;
}

int pick(const int*) {
  return 2;
}

int main() {
  first_like<int>::traits_type a = 0;
  second_like<int>::traits_type b = 0;
  return pick(a) * 10 + pick(b);
}
