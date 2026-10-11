template<class T>
T&& declval();

template<class F, class... Args>
auto my_invoke(F&&, Args&&...) -> decltype(declval<F>()(declval<Args>()...));

template<class F, class... Args>
struct invoke_result {
  typedef decltype(my_invoke(declval<F>(), declval<Args>()...)) type;
};

struct Fun {
  char operator()(int, long) const;
};

int main() {
  invoke_result<Fun&, int, long>::type c = char(0);
  return c;
}
