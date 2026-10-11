template<class U, class V, class T>
U const *f(T &, V *, U const *) { return 0; }

template<class U, class V, class T>
V *f(T &t, V *, ...) { return missing(t); }

struct V {};

int main() {
  V const v = V();
  V const *p = 0;
  return f<V>(v, p, p) != p;
}
