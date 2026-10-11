namespace std {
inline namespace __1 {
template<bool>
struct _BoolConstant {};

template<class T>
struct is_polymorphic {
  static const bool value = true;
};

template<class T, class U>
struct is_base_of {
  static const bool value = true;
};

template<class T, class U>
struct is_convertible {
  static const bool value = true;
};
}
}

namespace std {
template<class From, class To>
struct can_dynamic_cast
  : _BoolConstant<is_polymorphic<From>::value &&
                  (!is_base_of<To, From>::value ||
                   is_convertible<const From*, const To*>::value)> {};
}

int main() { return 0; }
