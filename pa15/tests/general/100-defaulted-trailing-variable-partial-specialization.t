template<class T, class U, class = void>
inline const bool same_v = false;

template<class T>
inline const bool same_v<T, T> = true;

static_assert(same_v<char, char>, "");

int main() {
  return 0;
}
