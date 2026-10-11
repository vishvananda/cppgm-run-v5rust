template<class T>
T&& declval();

template<class T>
void g(T&);

using result_t = decltype(g(declval<int&>()));

int main() {
  return 0;
}
