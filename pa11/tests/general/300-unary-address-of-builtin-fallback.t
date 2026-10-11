struct X {
  bool same(const X& other) const {
    return this == &other;
  }
};

struct Y {};
int operator&(Y, Y);

namespace address_of_fallback {
  struct regex_like {
    int value;
  };

  struct unrelated {
    int value;
  };

  int operator&(unrelated &value) {
    return value.value;
  }

  regex_like *addr(regex_like &value) {
    return &value;
  }
}

int main() {
  X x;
  if(!x.same(x)) {
    return 1;
  }

  address_of_fallback::regex_like value = {3};
  return address_of_fallback::addr(value) == &value ? 0 : 1;
}
