// Conditional explicitness belongs to the selected specialization, while unused
// conditions and bodies remain undemanded. expect exit 0
const bool condition = false;
template<class T> struct Box {
  int value;
  template<class U> explicit(condition || sizeof(U) > 4) Box(U v) : value(v) {}
  template<class U> explicit(U::missing) Box(U, U) {}
};
int take(Box<int> value) { return value.value; }

struct Choice {
  int value;
  template<class T, class U = typename T::missing>
  explicit(true) Choice(T, int) {}
  Choice(int v) : value(v) {}
};

int main() {
  const bool condition = true;
  Box<int> direct(7L);
  Box<int> copy = 3;
  Choice ordinary(5);
  return take(1) + direct.value + copy.value + ordinary.value - 16;
}
