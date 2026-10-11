// A local typedef in a function-template body retains the structured
// template-id on its elaborated type specifier when the body is instantiated.
namespace library {
namespace detail {

template<class T>
struct value_set {
  T value;
};

}

template<class T>
int read() {
  typedef struct detail::value_set<T> set_type;
  set_type value = {7};
  return value.value;
}

}

int main() {
  return library::read<int>() == 7 ? 0 : 1;
}
