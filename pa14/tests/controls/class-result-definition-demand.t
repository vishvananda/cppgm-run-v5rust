// expect exit 0
// Calls produce complete class objects whose temporary destructors are demanded;
// decltype's final call needs only its declared result identity.
int live;

template<class T> struct Value {
  Value() { ++live; }
  ~Value() { --live; }
};

template<class T> struct Result { Value<T> value; };
template<class T> Result<T> make() { return Result<T>(); }

template<class T> struct Pending { typename T::missing member; };
template<class T> Pending<T> declaration();
typedef decltype((declaration<int>())) Deferred;
Deferred* pending;

int main() {
  (void)make<int>();
  (void)make<long>();
  if (live != 0) return 1;
  return sizeof(pending) == sizeof(void*) ? 0 : 2;
}
