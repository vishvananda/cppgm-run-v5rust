namespace N {
template<class T>
inline const bool vt = true;

template<class T>
inline const bool vt<T&> = true;

template<class T>
inline const bool vt<T&&> = true;

template<class T, bool B = vt<T> >
class P {
  char arr[2];
};

template<class T>
class P<T, true> {};
}

struct Alloc {};

static_assert(sizeof(N::P<Alloc>) == 1, "");

int main() {
  return 0;
}
