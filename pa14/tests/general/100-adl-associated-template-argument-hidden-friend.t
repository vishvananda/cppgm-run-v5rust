// A type template argument contributes its class to the ADL associated set.
template<class T> struct box {};

struct item {
  friend int inspect(box<item>) { return 0; }
};

int main() {
  return inspect(box<item>());
}
