template<class T>
T&& __declval(int);

template<class T>
T __declval(long);

template<class T>
decltype(__declval<T>(0)) declval() {
  static_assert(sizeof(T) == 0, "declval body");
}

template<class T, class U>
struct common_type {
  typedef decltype(true ? declval<T>() : declval<U>()) type;
};

template<class T, class U, class V>
struct common_type3 {
  typedef typename common_type<typename common_type<T, U>::type, V>::type type;
};

typedef common_type3<long long, int, long>::type X;

int main() {
  return 0;
}
