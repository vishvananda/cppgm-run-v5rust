template<class T>
T&& __declval(int);

template<class T>
T&& __declval(long);

template<class T>
decltype(__declval<T>(0)) declval() {
  static_assert(sizeof(T) == 0, "declval body");
}

template<class T, class U>
struct common_type {
  typedef decltype(true ? declval<T>() : declval<U>()) type;
};

template<class R, class P>
struct duration {};

typedef common_type<duration<long long int, int>, duration<long long int, int> >::type X;

int main() {
  return 0;
}
