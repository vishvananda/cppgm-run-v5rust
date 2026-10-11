namespace support {
template<class T> struct decay { typedef T type; };
template<class T> using decay_t = typename decay<T>::type;
template<class T> T&& declval();
}

template<class Pointer, class = void>
struct AddressHelper;

template<class Pointer>
support::decay_t<decltype(AddressHelper<Pointer>::call(
    support::declval<const Pointer&>()))>
to_address(const Pointer& pointer) {
  return AddressHelper<Pointer>::call(pointer);
}

template<class Pointer, class>
struct AddressHelper {
  static int call(const Pointer&);
};

int main() { return 0; }
